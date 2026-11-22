// Profile editing in the Librarian Customize panel. Requests are serialized:
// typing waits 700 ms, edits made during a save follow it, and a late load
// cannot replace a draft the user has edited.
use crate::looks::{self, labeled, note, panel_section, switch_row};
use crate::style::px;
use crate::tips::Tip as _;
use crate::{app::Threadbear, call};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use threadbear_core::looks::profile::{
    self as rules, ProfileLink, ProfilePageSettings, ProfilePageStatus, ProfilePart,
};
use threadbear_core::looks::{LibrarianLook, Picture, PictureLook};

#[derive(Default)]
pub(crate) struct ProfileState {
    status: Option<ProfilePageStatus>,
    draft: Option<ProfilePageSettings>,
    loading: bool,
    loaded: bool,
    missing: bool,
    saving: bool,
    removing: bool,
    revision: u64,
    saved_revision: u64,
    writes_started: u64,
    pending: Option<Task<()>>,
    error: Option<String>,
    fields: HashMap<String, (Entity<InputState>, Subscription)>,
    #[cfg(feature = "automation")]
    load_delay_ms: u64,
}
impl Threadbear {
    pub(crate) fn published_profile_url(&self) -> Option<String> {
        let status = self.profile.status.as_ref()?;
        status
            .profile
            .as_ref()?
            .published
            .then(|| status.profile_url())
            .flatten()
    }
    pub(crate) fn load_profile_page(&mut self, cx: &mut Context<Self>) {
        if self.profile.loading || self.profile.saving || self.profile.removing {
            return;
        }
        self.profile.loading = true;
        self.profile.loaded = true;
        cx.notify();
        let client = self.client.clone();
        let revision = self.profile.revision;
        let writes_started = self.profile.writes_started;
        #[cfg(feature = "automation")]
        let load_delay_ms = std::mem::take(&mut self.profile.load_delay_ms);
        cx.spawn(async move |this, cx| {
            let got = call(client, "profilePage.get", json!({})).await;
            #[cfg(feature = "automation")]
            if load_delay_ms > 0 {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(load_delay_ms))
                    .await;
            }
            this.update(cx, |this, cx| {
                this.profile.loading = false;
                // A save can start after this read without another edit.
                // An old reply must not undo its result or clear its error.
                if this.profile.revision != revision
                    || this.profile.writes_started != writes_started
                {
                    cx.notify();
                    return;
                }
                match got {
                    Ok(v) => {
                        let status = ProfilePageStatus::decode(&v);
                        if this.profile.revision == revision
                            && this.profile.revision == this.profile.saved_revision
                            && this.profile.pending.is_none()
                            && !this.profile.saving
                        {
                            let mut draft = status.profile.clone().unwrap_or_default();
                            if status.profile.is_none() {
                                draft.name = crate::librarian_look::full_user_name();
                            }
                            this.profile.draft = Some(draft);
                            this.profile.fields.clear();
                        }
                        if this.profile.revision == revision {
                            this.profile.status = Some(status);
                        }
                        this.profile.missing = false;
                        this.profile.error = None;
                    }
                    Err(e) => {
                        this.profile.missing = e.code == -32601;
                        this.profile.error = Some(e.message);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub(crate) fn profile_look_changed(&mut self, cx: &mut Context<Self>) {
        if !self.profile.removing
            && self.profile.draft.is_some()
            && self
                .profile
                .status
                .as_ref()
                .is_some_and(|s| s.profile.is_some())
        {
            self.profile.revision += 1;
            self.profile_save_soon(cx);
        }
    }
    fn edit_profile(
        &mut self,
        cx: &mut Context<Self>,
        change: impl FnOnce(&mut ProfilePageSettings),
    ) {
        let Some(draft) = self.profile.draft.as_mut() else {
            return;
        };
        change(draft);
        self.profile.revision += 1;
        self.profile_save_soon(cx);
        cx.notify();
    }
    fn profile_save_soon(&mut self, cx: &mut Context<Self>) {
        self.profile.pending = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(700))
                .await;
            this.update(cx, |this, cx| {
                this.profile.pending = None;
                this.save_profile_page(cx);
            })
            .ok();
        }));
    }
    fn save_profile_page(&mut self, cx: &mut Context<Self>) {
        if self.profile.saving || self.profile.removing {
            return;
        }
        let Some(draft) = self.profile.draft.clone() else {
            return;
        };
        let revision = self.profile.revision;
        let look: LibrarianLook = self.look();
        let mut send = draft.sendable();
        send.order = rules::card_order(&look);
        send.github_user = (!look.github_user.is_empty()).then(|| look.github_user.clone());
        self.profile.saving = true;
        self.profile.writes_started += 1;
        self.profile.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            send.picture_path = cx
                .background_executor()
                .spawn(async move { profile_picture_path(&look) })
                .await;
            let got = call(client, "profilePage.update", send.to_value()).await;
            this.update(cx, |this, cx| {
                this.profile.saving = false;
                match got {
                    Ok(v) => {
                        this.profile.status = Some(ProfilePageStatus::decode(&v));
                        this.profile.saved_revision = revision;
                    }
                    Err(e) => {
                        this.profile.error = Some(e.message.clone());
                        this.show_error(e.message, cx);
                    }
                }
                // A new edit whose debounce has already fired while this
                // request was pending is sent next, never over the old one.
                if this.profile.revision > revision && this.profile.pending.is_none() {
                    this.save_profile_page(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn confirm_remove_profile_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let entity = entity.clone();
            alert.title("Take your profile page down?").description("Its page and pictures are deleted from your Convex backend. Your Librarian page stays as it is.").button_props(DialogButtonProps::default().ok_text("Take it down").show_cancel(true)).on_ok(move |_, _, cx| {
                entity.update(cx, |this, cx| this.remove_profile_page(cx)).ok();
                true
            })
        });
    }
    fn remove_profile_page(&mut self, cx: &mut Context<Self>) {
        if self.profile.saving || self.profile.removing {
            return;
        }
        self.profile.pending = None;
        self.profile.revision += 1;
        self.profile.removing = true;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let got = call(client, "profilePage.remove", json!({})).await;
            this.update(cx, |this, cx| {
                this.profile.removing = false;
                match got {
                    Ok(v) => {
                        this.profile.status = Some(ProfilePageStatus::decode(&v));
                        this.profile.draft = Some(ProfilePageSettings::default());
                        this.profile.fields.clear();
                        this.profile.saved_revision = this.profile.revision;
                        this.profile.error = None;
                    }
                    Err(e) => this.profile.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn profile_input(
        &mut self,
        key: &str,
        value: &str,
        placeholder: &str,
        tip: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if !self.profile.fields.contains_key(key) {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value.to_owned())
                    .placeholder(placeholder.to_owned())
            });
            let k = key.to_owned();
            let sub = cx.subscribe(&input, move |this, input, ev: &InputEvent, cx| {
                if !matches!(ev, InputEvent::Change) {
                    return;
                }
                let value = input.read(cx).value().to_string();
                this.edit_profile(cx, |draft| match k.as_str() {
                    "name" => draft.name = value,
                    "bio" => draft.bio = value.chars().take(160).collect(),
                    "repo" => draft.changelog_repo = (!value.is_empty()).then_some(value),
                    _ => {
                        let Some((field, index)) = k.split_once(':') else {
                            return;
                        };
                        let Some(link) = index
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| draft.links.get_mut(i))
                        else {
                            return;
                        };
                        if field == "title" {
                            link.title = value;
                        } else {
                            link.url = value;
                        }
                    }
                });
            });
            self.profile.fields.insert(key.to_owned(), (input, sub));
        }
        let input = self.profile.fields[key].0.clone();
        div()
            .id(SharedString::from(format!("profile-input-{key}")))
            .w(px(160.))
            .tip(tip)
            .child(Input::new(&input).small().disabled(self.profile.removing))
            .into_any_element()
    }
    #[expect(
        clippy::too_many_lines,
        reason = "baselined when the lint came in with Mike's 21 Nov 2026 release; split it when it's next worked on"
    )]
    pub(crate) fn render_profile_sections(
        &mut self,
        look: &LibrarianLook,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if !self.profile.loaded {
            let entity = cx.entity().downgrade();
            cx.defer(move |cx| {
                entity
                    .update(cx, |this, cx| this.load_profile_page(cx))
                    .ok();
            });
        }
        if self.profile.missing {
            return vec![panel_section(
                "Profile page",
                None,
                vec![note("Update the runner to make a profile page.", cx)],
                cx,
            )];
        }
        let Some(draft) = self.profile.draft.clone() else {
            let line = self
                .profile
                .error
                .clone()
                .unwrap_or_else(|| "Loading profile page…".into());
            return vec![panel_section(
                "Profile page",
                None,
                vec![
                    note(line, cx),
                    looks::kit_button(
                        "profile-load",
                        "Try again",
                        None,
                        "Load the saved profile page",
                        self.profile.loading,
                        self.profile.loading,
                        cx.listener(|this, _, _, cx| this.load_profile_page(cx)),
                    ),
                ],
                cx,
            )];
        };
        let locked = self.profile.removing;
        let mut rows = vec![switch_row(
            "profile-publish",
            "Publish my profile page",
            draft.published,
            "Public: anyone with the link can see it. Off: only your signed-in browser can see it",
            locked,
            cx,
            |this, on, cx| this.edit_profile(cx, |s| s.published = on),
        )];
        if draft.published {
            rows.push(switch_row(
                "profile-home",
                "Show on my site's front page",
                draft.home,
                "Visitors see this page first. Signed in, you still get your threads there",
                locked,
                cx,
                |this, on, cx| this.edit_profile(cx, |s| s.home = on),
            ));
        }
        if let Some(status) = &self.profile.status {
            if let Some(url) = status.profile_url() {
                rows.push(profile_address(
                    "profile",
                    "Profile page",
                    &url,
                    "Opens your profile, signed in or not.",
                    cx,
                ));
            }
            if let Some(url) = status.site_url() {
                rows.push(profile_address(
                    "site",
                    "Front page",
                    &url,
                    "Visitors see your profile here. Signed in, you see your threads.",
                    cx,
                ));
            }
        }
        if draft.published {
            rows.push(switch_row(
                "profile-view-button",
                "View profile page button",
                look.profile_button,
                "Show View profile page beside Customize on the Librarian page",
                false,
                cx,
                |this, on, cx| this.edit_look::<LibrarianLook>(cx, |l| l.profile_button = on),
            ));
        }
        for (key, label, value, placeholder, tip) in [
            (
                "name",
                "Name",
                draft.name.as_str(),
                "Your name",
                "The name at the top of your page",
            ),
            (
                "bio",
                "Bio",
                draft.bio.as_str(),
                "One line about you",
                "A line under your name, up to 160 characters",
            ),
        ] {
            let input = self.profile_input(key, value, placeholder, tip, window, cx);
            rows.push(labeled(label, input, cx));
        }
        for (i, link) in draft.links.iter().enumerate() {
            let title = self.profile_input(
                &format!("title:{i}"),
                &link.title,
                "Title",
                "What the link says on your page",
                window,
                cx,
            );
            let url = self.profile_input(
                &format!("url:{i}"),
                &link.url,
                "https://",
                "An http or https address. It shows once the address is complete",
                window,
                cx,
            );
            rows.push(
                v_flex()
                    .gap(px(6.))
                    .child(title)
                    .child(url)
                    .child(looks::kit_button(
                        format!("profile-link-remove-{i}"),
                        "Remove link",
                        None,
                        "Take this link off your page",
                        locked,
                        false,
                        cx.listener(move |this, _, _, cx| {
                            this.edit_profile(cx, |s| {
                                s.links.remove(i);
                            });
                            this.profile.fields.clear();
                        }),
                    ))
                    .into_any_element(),
            );
        }
        if draft.links.len() < rules::LINKS_MAX {
            rows.push(looks::kit_button(
                "profile-add-link",
                "Add a link",
                None,
                "Add up to four links to your page",
                locked,
                false,
                cx.listener(|this, _, _, cx| {
                    this.edit_profile(cx, |s| s.links.push(ProfileLink::default()))
                }),
            ));
        }
        let mut sections = vec![panel_section("Profile page", Some(("profile-info", "A page about you on your own Threadbear site, kept on your Convex backend. It saves as you change it. Thread titles, prompts, replies, folders and project notes are never sent, and threads in a private folder aren't counted.")), rows, cx)];
        let rows = rules::offered()
            .into_iter()
            .filter(|p| *p != ProfilePart::Changelog)
            .map(|part| {
                switch_row(
                    format!("profile-part-{}", part.raw()),
                    part.title(),
                    draft.shows(part),
                    part.tip(),
                    locked,
                    cx,
                    move |this, on, cx| this.edit_profile(cx, |s| s.set_shown(on, part)),
                )
            })
            .collect();
        sections.push(panel_section("What's on it", None, rows, cx));
        if draft.shows(ProfilePart::Numbers) {
            let rows = rules::NUMBER_CHOICES
                .iter()
                .map(|&(key, label)| {
                    switch_row(
                        format!("profile-number-{key}"),
                        label,
                        draft.numbers.iter().any(|n| n == key),
                        format!("Show {} on your page", label.to_lowercase()),
                        locked,
                        cx,
                        move |this, on, cx| {
                            this.edit_profile(cx, |s| {
                                s.numbers.retain(|n| n != key);
                                if on {
                                    s.numbers.push(key.into());
                                }
                            })
                        },
                    )
                })
                .collect();
            sections.push(panel_section("Numbers on your page", None, rows, cx));
        }
        if draft.shows(ProfilePart::Projects) {
            let projects: Vec<_> = self
                .projects
                .list
                .iter()
                .filter(|p| !p.archived)
                .map(|p| (p.id.clone(), p.title.clone()))
                .collect();
            let mut rows = vec![];
            for (id, title) in projects {
                let on = draft.project_ids.contains(&id);
                rows.push(switch_row(
                    format!("profile-project-{id}"),
                    title,
                    on,
                    "Show only this project's title and thread count",
                    locked || (!on && draft.project_ids.len() >= 12),
                    cx,
                    move |this, on, cx| {
                        this.edit_profile(cx, |s| {
                            s.project_ids.retain(|p| p != &id);
                            if on {
                                s.project_ids.push(id.clone());
                            }
                        })
                    },
                ));
            }
            if rows.is_empty() {
                rows.push(note("No projects yet.", cx));
            }
            sections.push(panel_section("Projects on your page", None, rows, cx));
        }
        let mut rows = vec![switch_row(
            "profile-changelog",
            "Show my changelog on my page",
            draft.shows(ProfilePart::Changelog),
            ProfilePart::Changelog.tip(),
            locked,
            cx,
            |this, on, cx| this.edit_profile(cx, |s| s.set_shown(on, ProfilePart::Changelog)),
        )];
        if (draft.shows(ProfilePart::Github) || draft.shows(ProfilePart::Changelog))
            && look.github_user.is_empty()
        {
            rows.push(note(
                "Type your GitHub user name under GitHub activity above.",
                cx,
            ));
        }
        if draft.shows(ProfilePart::Changelog) {
            let input = self.profile_input("repo", draft.changelog_repo.as_deref().unwrap_or(""), "owner/threadbear", "A public GitHub repository, owner/name. Leave empty for your GitHub user name/threadbear", window, cx);
            rows.push(labeled("Repository", input, cx));
            rows.push(note(
                draft
                    .changelog_repo_for(&look.github_user)
                    .map(|r| {
                        format!(
                            "Public commits by @{} to {r}, newest first.",
                            look.github_user
                        )
                    })
                    .unwrap_or_else(|| "Type a repository as owner/name.".into()),
                cx,
            ));
        }
        sections.push(panel_section("Changelog", None, rows, cx));
        let busy = self.profile.saving || self.profile.pending.is_some() || self.profile.removing;
        let line = if busy {
            "Saving…".to_owned()
        } else if let Some(e) = self.profile.error.as_ref().or_else(|| {
            self.profile
                .status
                .as_ref()
                .and_then(|s| s.problem.as_ref())
        }) {
            e.clone()
        } else if self
            .profile
            .status
            .as_ref()
            .is_none_or(|s| s.profile.is_none())
        {
            "Nothing's sent until you change something here.".into()
        } else if draft.published {
            "Saved and public. Its numbers refresh within the hour.".into()
        } else {
            "Saved. Only you can see it until you publish it.".into()
        };
        let mut rows = vec![note(line, cx)];
        if self.profile.error.is_some() {
            rows.push(looks::kit_button(
                "profile-retry",
                "Try again",
                None,
                "Save the profile page again",
                busy,
                busy,
                cx.listener(|this, _, _, cx| this.save_profile_page(cx)),
            ));
        }
        if self
            .profile
            .status
            .as_ref()
            .is_some_and(|s| s.profile.is_some())
        {
            rows.push(looks::kit_button(
                "profile-remove",
                "Take my profile page down",
                None,
                "Delete the page and its pictures from your Convex backend",
                busy,
                busy,
                cx.listener(|this, _, window, cx| this.confirm_remove_profile_page(window, cx)),
            ));
        }
        sections.push(panel_section("", None, rows, cx));
        sections
    }
    #[cfg(feature = "automation")]
    pub(crate) fn profile_dump(&self) -> Value {
        json!({ "loaded": self.profile.loaded && !self.profile.loading, "saving": self.profile.saving || self.profile.pending.is_some(), "draft": self.profile.draft.as_ref().map(ProfilePageSettings::to_value), "profileURL": self.profile.status.as_ref().and_then(ProfilePageStatus::profile_url), "siteURL": self.profile.status.as_ref().and_then(ProfilePageStatus::site_url), "problem": self.profile.error, "saved": self.profile.status.as_ref().and_then(|s| s.profile.as_ref()).map(ProfilePageSettings::to_value) })
    }
}
fn profile_picture_path(look: &LibrarianLook) -> Option<String> {
    match look.pic().picture {
        Picture::None => None,
        Picture::Own => looks::path(look.pic().picture_file.as_deref()?)
            .map(|p| p.to_string_lossy().into_owned()),
        Picture::Bear => {
            let file = looks::folder().join("profile-bear.png");
            if !file.exists() {
                let image =
                    image::load_from_memory(include_bytes!("../../../shared/art/librarian.png"))
                        .ok()?
                        .thumbnail(480, 480);
                let mut bytes = Vec::new();
                image
                    .write_to(
                        &mut std::io::Cursor::new(&mut bytes),
                        image::ImageFormat::Png,
                    )
                    .ok()?;
                threadbear_core::private_file::write(&file, &bytes).ok()?;
            }
            Some(file.to_string_lossy().into_owned())
        }
    }
}
fn profile_address(
    id: &'static str,
    title: &'static str,
    url: &str,
    explanation: &'static str,
    cx: &App,
) -> AnyElement {
    let (copy, open) = (url.to_owned(), url.to_owned());
    v_flex()
        .gap(px(4.))
        .child(
            h_flex()
                .gap(px(6.))
                .child(div().flex_1().child(title))
                .child(looks::kit_button(
                    format!("profile-{id}-copy"),
                    "Copy",
                    None,
                    "Copy this address",
                    false,
                    false,
                    move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())),
                ))
                .child(looks::kit_button(
                    format!("profile-{id}-open"),
                    "Open",
                    None,
                    "Open this address in the browser",
                    false,
                    false,
                    move |_, _, cx| {
                        crate::open_gate::open_web(&open, cx);
                    },
                )),
        )
        .child(note(url.to_owned(), cx))
        .child(note(explanation, cx))
        .into_any_element()
}

#[cfg(feature = "automation")]
pub(crate) fn automation(
    app: &Entity<Threadbear>,
    cmd: &Value,
    cx: &mut App,
) -> Result<Value, String> {
    app.update(cx, |this, cx| {
        if cmd["load"].as_bool() == Some(true) {
            this.profile.load_delay_ms = cmd["delayMs"].as_u64().unwrap_or(0);
            this.load_profile_page(cx);
        }
        if let Some(set) = cmd["set"].as_object() {
            if this.profile.draft.is_none() {
                return Err("load the profile page first".to_owned());
            }
            let set = set.clone();
            this.edit_profile(cx, |draft| {
                let mut value = draft.to_value();
                for (key, v) in set {
                    value[key] = v;
                }
                *draft = ProfilePageSettings::decode(&value);
            });
            this.profile.fields.clear();
        }
        Ok(json!("ok"))
    })
}
