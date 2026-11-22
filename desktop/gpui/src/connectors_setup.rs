// Setup and Outside agents use the existing runner RPCs. The panel only keeps
// the new secret until Done or close; saved settings contain hidden row ids.
use crate::activity_panels::{
    about_button_with_tip, icon_button, push_button, small_heading, system_orange, text_line,
    MenuLabel,
};
use crate::app::Threadbear;
use crate::highlights::{tipped, tipped_as};
use crate::style::px;
use crate::{call, now_ms, shell, style};
use gpui_kit::component::button::Button;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use threadbear_core::connectors::{
    self as rules, AgentConnected, AgentsList, ClientSetup, ConnectorCatalogEntry,
    ConnectorsStatus, LocalServer, OutsideAgentApp,
};

#[derive(Default)]
pub(crate) struct SetupState {
    pub(crate) agents: Option<AgentsList>,
    busy: HashSet<String>,
    made: Option<AgentConnected>,
    picked: OutsideAgentApp,
    write: bool,
    read_all: bool,
    agent_form: bool,
    custom: Option<CustomServerForm>,
    trust: Option<LocalTrust>,
    client: Option<ClientForm>,
    // Closing and reopening during an RPC must not resurrect a one-time key.
    epoch: u64,
    list_request: u64,
}

struct CustomServerForm {
    local: bool,
    npx: bool,
    address: Entity<InputState>,
    version: Entity<InputState>,
    title: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}
impl CustomServerForm {
    fn server(&self, cx: &App) -> LocalServer {
        LocalServer::new(
            if self.npx { "npx" } else { "uvx" },
            &self.address.read(cx).value(),
            &self.version.read(cx).value(),
            &self.title.read(cx).value(),
        )
    }
}

/// The paste form for a service's own OAuth app (Slack's Client ID and
/// Secret). The secret lives only in its masked field, until Save sends it
/// in the one `connectors.saveClient` call, or Cancel or close drops it.
struct ClientForm {
    id: String,
    client_id: Entity<InputState>,
    secret: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct LocalTrust {
    title: String,
    command: String,
    asks: Vec<String>,
    note: Option<String>,
    params: Value,
    busy_key: String,
}
impl LocalTrust {
    fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Add {} to Connectors?", self.title),
            "This command will run on this Mac:".into(),
            self.command.clone(),
        ];
        lines.extend(rules::trust_lines(&self.asks));
        lines.extend(self.note.clone());
        lines
    }
}

impl Threadbear {
    pub(crate) fn hidden_connectors(&self) -> BTreeSet<String> {
        rules::hidden_decode(
            self.settings
                .saved
                .connectors_hidden
                .as_deref()
                .unwrap_or_default(),
        )
    }
    pub(crate) fn visible_connectors(&self) -> Option<ConnectorsStatus> {
        rules::visible_status(
            self.connectors_panel.status.as_ref(),
            &self.hidden_connectors(),
        )
    }

    fn change_connector_hidden(&mut self, ids: &[String], hide: bool, cx: &mut Context<Self>) {
        let mut hidden = self.hidden_connectors();
        for id in ids {
            if hide {
                hidden.insert(id.clone());
            } else {
                hidden.remove(id);
            }
        }
        self.settings.saved.connectors_hidden = Some(rules::hidden_encode(&hidden));
        self.save_settings_soon(cx);
        cx.notify();
    }

    pub(crate) fn close_connector_forms(&mut self) {
        let state = &mut self.connectors_panel.setup;
        state.made = None;
        state.agent_form = false;
        state.custom = None;
        state.trust = None;
        state.client = None;
        state.epoch += 1;
    }

    pub(crate) fn load_outside_agents(&mut self, cx: &mut Context<Self>) {
        self.connectors_panel.setup.list_request += 1;
        let request = self.connectors_panel.setup.list_request;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "agents.list", json!({})).await;
            this.update(cx, |this, cx| {
                if this.connectors_panel.setup.list_request != request {
                    return;
                }
                match result {
                    Ok(v) => this.connectors_panel.setup.agents = Some(AgentsList::decode(&v)),
                    Err(e) if e.code == -32601 => this.connectors_panel.setup.agents = None,
                    // This refresh also runs at hello: keep the last list and
                    // log a transient outage without interrupting other work.
                    Err(e) => threadbear_client::log::write(
                        "rpc",
                        &format!("agents.list failed: {}", e.message),
                    ),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn start_connector_thread(
        &mut self,
        method: &'static str,
        params: Value,
        key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.connectors_panel.setup.busy.insert(key.clone()) {
            return;
        }
        cx.notify();
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, method, params).await;
            this.update_in(cx, |this, window, cx| {
                this.connectors_panel.setup.busy.remove(&key);
                match result {
                    Ok(v) => match v["threadId"].as_str().filter(|s| !s.is_empty()) {
                        Some(id) => this.open_started_thread(id.to_owned(), window, cx),
                        None => {
                            this.show_error("The runner didn't return the setup thread.".into(), cx)
                        }
                    },
                    Err(e) => this.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn make_agent_key(&mut self, cx: &mut Context<Self>) {
        let s = &mut self.connectors_panel.setup;
        if !s.picked.can_connect() || !s.busy.insert("make-key".into()) {
            return;
        }
        // An older list response must not erase the key that is being made.
        s.list_request += 1;
        let epoch = s.epoch;
        let params = json!({"name":s.picked.name(),"scope":if s.write {"write"} else {"read"},"readAll":s.read_all});
        let client = self.client.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = call(client, "agents.connect", params).await;
            this.update(cx, |this, cx| {
                this.connectors_panel.setup.busy.remove("make-key");
                match result {
                    Ok(v) => {
                        if let Some(made) = AgentConnected::decode(&v) {
                            let shown = this.activity_panel_showing(shell::CONNECTORS, cx);
                            let s = &mut this.connectors_panel.setup;
                            if let Some(agents) = &mut s.agents {
                                agents.keys.push(made.key.clone());
                            }
                            if s.epoch == epoch && shown {
                                s.made = Some(made);
                                s.agent_form = false;
                            }
                            this.load_outside_agents(cx);
                        } else {
                            this.show_error(
                                "The runner didn't return a complete agent key.".into(),
                                cx,
                            );
                        }
                    }
                    Err(e) => this.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn revoke_agent_key(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let busy = format!("revoke:{}", id.as_deref().unwrap_or("all"));
        if !self.connectors_panel.setup.busy.insert(busy.clone()) {
            return;
        }
        self.connectors_panel.setup.list_request += 1;
        let client = self.client.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = call(
                client,
                "agents.revoke",
                id.as_ref().map_or_else(|| json!({}), |id| json!({"id":id})),
            )
            .await;
            this.update(cx, |this, cx| {
                let s = &mut this.connectors_panel.setup;
                s.busy.remove(&busy);
                match result {
                    Ok(v) => {
                        s.agents = Some(AgentsList::decode(&v));
                        if s.made
                            .as_ref()
                            .is_some_and(|m| id.as_ref().is_none_or(|id| id == &m.key.id))
                        {
                            s.made = None;
                        }
                        this.load_outside_agents(cx);
                    }
                    Err(e) => this.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn render_connector_setup(&mut self, cx: &mut Context<Self>) -> AnyElement {
        v_flex().gap(px(8.))
            .child(text_line("Share one sign-in for Notion, Slack and other services with every login and agent, with a check before anything is sent.", 12., style::label_text(cx)))
            .child(h_flex().gap(px(8.)).child(push_button("connectors-setup", "Set up", "Start a thread that installs the Connectors gateway on this Mac", true, self.connectors_panel.setup.busy.contains("setup"), cx, |t, w, cx| t.start_connector_thread("connectors.setup", json!({}), "setup".into(), w, cx)))
            .child(about_button_with_tip("connectors-setup-about", "What Set up does", vec![
                "A thread installs plugbear, a small gateway of your own, on this Mac. It listens only on this Mac, at 127.0.0.1:8795.".into(),
                "It wires each of your logins to it, so a service signed in once works for all of them, and Codex and the Grok CLI too.".into(),
                "Sends, posts and deletes wait for your OK on its approval page, and it keeps a log of every call. Sign-ins stay in the gateway, never in Threadbear.".into(),
                "The thread asks you before anything that needs a password, and says what it changed.".into(),
            ], None, cx))).into_any_element()
    }

    pub(crate) fn connector_hide_button(
        &mut self,
        id: String,
        tip: String,
        group: SharedString,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Keep the hit target and accessibility node even while the glyph is hidden.
        tipped(
            div()
                .id(SharedString::from(format!("connector-hide-{id}")))
                .size(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer(),
            tip,
        )
        .on_click(cx.listener(move |t, _: &ClickEvent, _, cx| {
            t.change_connector_hidden(std::slice::from_ref(&id), true, cx)
        }))
        .child(
            div()
                .opacity(0.)
                .group_hover(group, |s| s.opacity(1.))
                .child(crate::icons::symbol("xmark").with_size(px(10.))),
        )
        .into_any_element()
    }

    pub(crate) fn render_hidden_connectors(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let items = rules::hidden_items(
            &self.hidden_connectors(),
            self.connectors_panel.status.as_ref(),
            self.connectors_panel.setup.agents.as_ref(),
        );
        if items.is_empty() {
            return None;
        }
        let app = cx.entity().downgrade();
        let menu = MenuLabel::new(
            "connectors-hidden-menu",
            format!("+ {} hidden", items.len()),
            11.,
            style::secondary_text(cx),
        )
        .dropdown_menu(move |menu: PopupMenu, _, _| {
            let mut menu = menu;
            for (id, title) in &items {
                let (app, id) = (app.clone(), id.clone());
                menu = menu.item(PopupMenuItem::new(format!("Show {title}")).on_click(
                    move |_, _, cx| {
                        app.update(cx, |t, cx| {
                            t.change_connector_hidden(std::slice::from_ref(&id), false, cx)
                        })
                        .ok();
                    },
                ));
            }
            let app = app.clone();
            let ids: Vec<_> = items.iter().map(|(id, _)| id.clone()).collect();
            menu.item(PopupMenuItem::new("Show all").on_click(move |_, _, cx| {
                app.update(cx, |t, cx| t.change_connector_hidden(&ids, false, cx))
                    .ok();
            }))
        });
        Some(
            tipped_as(
                div().id("connectors-hidden"),
                "Bring back a connector you hid",
                Role::GenericContainer,
            )
            .child(menu)
            .into_any_element(),
        )
    }

    pub(crate) fn render_connector_sections(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut sections = vec![];
        if let Some(status) = self.visible_connectors().filter(|s| s.state == "ok") {
            // Keep Other MCP server reachable even after every suggestion was hidden.
            if self
                .connectors_panel
                .status
                .as_ref()
                .is_some_and(|s| !s.catalog.is_empty() || s.can_add_local)
            {
                let mut rows = vec![small_heading("Add a service", cx).into_any_element()];
                for entry in &status.catalog {
                    rows.push(self.render_catalog_connector(entry, cx));
                }
                if self.connectors_panel.setup.custom.is_some() {
                    rows.push(self.render_custom_connector(cx));
                } else {
                    rows.push(push_button(
                        "connectors-other",
                        "Other MCP server...",
                        "Add any MCP server by its https address",
                        true,
                        false,
                        cx,
                        |t, w, cx| t.open_custom_connector(false, w, cx),
                    ));
                    if status.can_add_local {
                        rows.push(push_button("connectors-other-local", "Other local server...", "Add an MCP server that runs on this Mac, from a uvx or npx package at one exact version", true, false, cx, |t, w, cx| t.open_custom_connector(true, w, cx)));
                    }
                }
                sections.push(section(rows, cx));
            }
        }
        if rules::show_agents(
            self.connectors_panel.setup.agents.as_ref(),
            &self.hidden_connectors(),
        ) {
            sections.push(self.render_outside_agents(window, cx));
        }
        sections
    }

    fn render_catalog_connector(
        &mut self,
        entry: &ConnectorCatalogEntry,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let group = SharedString::from(format!("catalog-{}", entry.id));
        let mut words = v_flex().flex_1().min_w_0().gap(px(2.)).child(text_line(
            entry.title.clone(),
            12.,
            style::label_text(cx),
        ));
        if entry.is_local() {
            words = words.child(text_line("On this Mac", 10., style::secondary_text(cx)));
        }
        if !entry.blurb.is_empty() {
            words = words.child(text_line(
                entry.blurb.clone(),
                11.,
                style::secondary_text(cx),
            ));
        }
        let mut row = h_flex()
            .group(group.clone())
            .items_start()
            .gap(px(8.))
            .child(words);
        if let Some(note) = &entry.setup_note {
            row = row.child(about_button_with_tip(
                SharedString::from(format!("catalog-note-{}", entry.id)),
                format!("What {} needs first", entry.title),
                vec![note.clone()],
                None,
                cx,
            ));
        }
        if let Some(setup) = entry.client_setup.clone() {
            row = row.child(self.create_app_button(&entry.id, &entry.title, &setup.create_url, cx));
            let row = row.child(self.connector_hide_button(
                format!("catalog:{}", entry.id),
                format!(
                    "Hide the {} suggestion. The + below brings it back.",
                    entry.title
                ),
                group,
                cx,
            ));
            return v_flex()
                .gap(px(8.))
                .child(row)
                .child(self.render_client_setup(&entry.id, &entry.title, &setup, cx))
                .into_any_element();
        }
        let e = entry.clone();
        let busy = self.connectors_panel.setup.busy.contains(&entry.id);
        row = row.child(push_button(
            SharedString::from(format!("catalog-add-{}", entry.id)),
            if entry.is_local() { "Add..." } else { "Add" },
            if entry.is_local() {
                format!(
                    "See the exact command {} runs before adding it",
                    entry.title
                )
            } else {
                format!("Start a thread that adds {} to the gateway", entry.title)
            },
            true,
            busy,
            cx,
            move |t, w, cx| {
                if e.is_local() {
                    if let Some(command) = e.command.clone().filter(|s| !s.is_empty()) {
                        t.open_local_trust(
                            LocalTrust {
                                title: e.title.clone(),
                                command,
                                asks: e.asks.clone(),
                                note: e.setup_note.clone(),
                                params: json!({"id":e.id}),
                                busy_key: e.id.clone(),
                            },
                            w,
                            cx,
                        );
                    } else {
                        t.show_error(
                            "The runner hasn't supplied this server's exact command.".into(),
                            cx,
                        );
                    }
                } else {
                    t.start_connector_thread(
                        "connectors.add",
                        json!({"id":e.id}),
                        e.id.clone(),
                        w,
                        cx,
                    );
                }
            },
        ));
        row.child(self.connector_hide_button(
            format!("catalog:{}", entry.id),
            format!(
                "Hide the {} suggestion. The + below brings it back.",
                entry.title
            ),
            group,
            cx,
        ))
        .into_any_element()
    }

    /// `Create Slack app`: opens the service's page that makes the app,
    /// filled in, and the paste form for what it gives back.
    pub(crate) fn create_app_button(
        &mut self,
        id: &str,
        title: &str,
        url: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (id, url) = (id.to_owned(), url.to_owned());
        push_button(
            SharedString::from(format!("connector-create-{id}")),
            rules::create_app_title(title),
            format!("Open {title} with the app filled in, then paste its keys here"),
            true,
            false,
            cx,
            move |t, w, cx| {
                crate::open_gate::open_web(&url, cx);
                t.open_client_form(&id, w, cx);
            },
        )
    }

    fn open_client_form(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .connectors_panel
            .setup
            .client
            .as_ref()
            .is_some_and(|f| f.id == id)
        {
            return;
        }
        let client_id = cx.new(|cx| InputState::new(window, cx).placeholder("Client ID"));
        let secret = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Client Secret");
            state.set_masked(true, window, cx);
            state
        });
        // A pasted secret not yet saved: an update's restart waits for it.
        crate::unsent::watch(&secret, "an app's Client Secret field", cx);
        let mut subscriptions: Vec<Subscription> = [&client_id, &secret]
            .into_iter()
            .map(|input| cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()))
            .collect();
        let who = id.to_owned();
        subscriptions.push(cx.subscribe_in(
            &secret,
            window,
            move |t, _, event: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    t.save_client(&who, cx);
                }
            },
        ));
        let focus = client_id.read(cx).focus_handle(cx);
        self.connectors_panel.setup.client = Some(ClientForm {
            id: id.to_owned(),
            client_id,
            secret,
            _subscriptions: subscriptions,
        });
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        cx.notify();
    }

    /// `connectors.saveClient`: the runner writes the keys where the gateway
    /// reads them, turns the service on and restarts the gateway, and
    /// answers with the panel's new status once the gateway is back.
    fn save_client(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(form) = self
            .connectors_panel
            .setup
            .client
            .as_ref()
            .filter(|f| f.id == id)
        else {
            return;
        };
        let (client_id, secret) = (
            form.client_id.read(cx).value().trim().to_owned(),
            form.secret.read(cx).value().trim().to_owned(),
        );
        if !rules::paste_check(id, &client_id, &secret).can_save
            || !self
                .connectors_panel
                .setup
                .busy
                .insert("save-client".into())
        {
            return;
        }
        let epoch = self.connectors_panel.setup.epoch;
        let params = json!({ "id": id, "clientId": client_id, "clientSecret": secret });
        let client = self.client.clone();
        let id = id.to_owned();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = call(client, "connectors.saveClient", params).await;
            this.update(cx, |t, cx| {
                t.connectors_panel.setup.busy.remove("save-client");
                match result {
                    Ok(v) => {
                        t.connectors_panel.status =
                            Some(ConnectorsStatus::decode(&v["connectors"]));
                        if t.connectors_panel.setup.epoch == epoch
                            && t.connectors_panel
                                .setup
                                .client
                                .as_ref()
                                .is_some_and(|f| f.id == id)
                        {
                            t.connectors_panel.setup.client = None;
                        }
                        t.flash_done("Saved. Press Connect to sign in", cx);
                    }
                    Err(e) => t.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Under a service that needs its own app: a line on what Create does,
    /// and once it's pressed (or Paste keys), the two fields and Save.
    pub(crate) fn render_client_setup(
        &mut self,
        id: &str,
        title: &str,
        setup: &ClientSetup,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let mut rows = vec![];
        if !setup.help.is_empty() {
            rows.push(text_line(setup.help.clone(), 11., secondary).into_any_element());
        }
        let open = self
            .connectors_panel
            .setup
            .client
            .as_ref()
            .filter(|f| f.id == id);
        let Some(form) = open else {
            let who = id.to_owned();
            rows.push(push_button(
                SharedString::from(format!("connector-paste-{id}")),
                "Paste keys",
                format!("Already made the {title} app? Paste its Client ID and Secret"),
                true,
                false,
                cx,
                move |t, w, cx| t.open_client_form(&who, w, cx),
            ));
            return v_flex().gap(px(6.)).children(rows).into_any_element();
        };
        let (client_id, secret) = (form.client_id.clone(), form.secret.clone());
        let check = rules::paste_check(id, &client_id.read(cx).value(), &secret.read(cx).value());
        rows.push(input_row(
            "connector-client-id",
            "Client ID",
            &client_id,
            cx,
        ));
        rows.push(input_row(
            "connector-client-secret",
            "Client Secret",
            &secret,
            cx,
        ));
        if let Some(warning) = check.warning.clone() {
            rows.push(text_line(warning, 11., system_orange(cx)).into_any_element());
        }
        let busy = self.connectors_panel.setup.busy.contains("save-client");
        let mut buttons = h_flex().gap(px(6.));
        if check.can_save {
            let who = id.to_owned();
            buttons = buttons.child(push_button(
                "connector-client-save",
                "Save",
                format!("Save the keys to the gateway, turn {title} on and restart the gateway"),
                true,
                busy,
                cx,
                move |t, _, cx| t.save_client(&who, cx),
            ));
        } else {
            buttons = buttons.child(
                tipped(
                    div()
                        .id("connector-client-save-off")
                        .opacity(0.4)
                        .text_size(px(11.)),
                    "Paste both the Client ID and the Client Secret",
                )
                .child("Save"),
            );
        }
        buttons = buttons.child(push_button(
            "connector-client-cancel",
            "Cancel",
            "Put the form away. Nothing is saved",
            true,
            false,
            cx,
            |t, _, cx| {
                t.connectors_panel.setup.client = None;
                cx.notify();
            },
        ));
        rows.push(buttons.into_any_element());
        if !setup.redirect_url.is_empty() {
            rows.push(
                text_line(
                    format!("The app signs in back to {}.", setup.redirect_url),
                    11.,
                    style::tertiary_label(cx),
                )
                .into_any_element(),
            );
        }
        v_flex().gap(px(6.)).children(rows).into_any_element()
    }

    fn open_custom_connector(&mut self, local: bool, window: &mut Window, cx: &mut Context<Self>) {
        let address = cx.new(|cx| {
            InputState::new(window, cx).placeholder(if local {
                "Package"
            } else {
                "https://example.com/mcp"
            })
        });
        let version = cx.new(|cx| InputState::new(window, cx).placeholder("Version, like 1.2.3"));
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let subscriptions = [&address, &version, &title]
            .into_iter()
            .map(|input| cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()))
            .collect();
        let focus = address.read(cx).focus_handle(cx);
        self.connectors_panel.setup.custom = Some(CustomServerForm {
            local,
            npx: false,
            address,
            version,
            title,
            _subscriptions: subscriptions,
        });
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        cx.notify();
    }

    #[expect(
        clippy::unwrap_used,
        reason = "only drawn while what it unwraps is set, just above or by its caller"
    )]
    fn render_custom_connector(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let form = self.connectors_panel.setup.custom.as_ref().unwrap();
        let mut rows = vec![];
        if form.local {
            let selected_npx = form.npx;
            rows.push(
                h_flex()
                    .gap(px(6.))
                    .children([false, true].map(|npx| {
                        div()
                            .when(npx == selected_npx, |d| {
                                d.border_b_2().border_color(style::accent(cx))
                            })
                            .child(push_button(
                                if npx { "local-npx" } else { "local-uvx" },
                                if npx { "npx (Node)" } else { "uvx (Python)" },
                                if npx {
                                    "Run an npm package"
                                } else {
                                    "Run a PyPI package"
                                },
                                true,
                                false,
                                cx,
                                move |t, _, cx| {
                                    if let Some(f) = &mut t.connectors_panel.setup.custom {
                                        f.npx = npx;
                                    }
                                    cx.notify();
                                },
                            ))
                    }))
                    .into_any_element(),
            );
        }
        // Read again after button closures are built; fields stay alive across renders.
        let form = self.connectors_panel.setup.custom.as_ref().unwrap();
        rows.push(input_row(
            "connector-address",
            if form.local { "Package" } else { "Server URL" },
            &form.address,
            cx,
        ));
        if form.local {
            rows.push(input_row(
                "connector-version",
                "Exact version",
                &form.version,
                cx,
            ));
        }
        rows.push(input_row("connector-title", "Name", &form.title, cx));
        let server = form.server(cx);
        let local = form.local;
        let url = form.address.read(cx).value().trim().to_owned();
        let title = form.title.read(cx).value().to_string();
        let valid = if local {
            server.is_pinned()
        } else {
            url.to_lowercase().starts_with("https://")
        };
        let busy_key = if local { server.key() } else { url.clone() };
        let busy = self.connectors_panel.setup.busy.contains(&busy_key);
        let mut buttons = h_flex().gap(px(6.));
        if valid {
            buttons = buttons.child(push_button("connector-custom-add", if local {"Add..."} else {"Add"}, if local {"See the exact command before anything is added"} else {"Start a thread that adds this server to the gateway"}, true, busy, cx, move |t,w,cx| {
                if local { t.open_local_trust(LocalTrust {title:if server.title.is_empty(){server.package.clone()}else{server.title.clone()},command:server.command(),asks:vec![],note:None,params:json!({"local":{"launcher":server.launcher,"package":server.package,"version":server.version,"title":server.title}}),busy_key:server.key()},w,cx); }
                else {t.start_connector_thread("connectors.add",json!({"url":url,"title":title}),url.clone(),w,cx);}
            }));
        } else {
            buttons = buttons.child(
                tipped(
                    div()
                        .id("connector-custom-invalid")
                        .opacity(0.4)
                        .text_size(px(11.)),
                    if local {
                        "Needs a package and one exact version"
                    } else {
                        "Needs a public https address"
                    },
                )
                .child(if local { "Add..." } else { "Add" }),
            );
        }
        buttons = buttons.child(push_button(
            "connector-custom-cancel",
            "Cancel",
            "Don't add a server",
            true,
            false,
            cx,
            |t, _, cx| {
                t.connectors_panel.setup.custom = None;
                cx.notify();
            },
        ));
        rows.push(buttons.into_any_element());
        rows.push(text_line(if local {"Only uvx and npx packages at one exact version. The sheet shows the command before anything runs."} else {"Only public https servers. Anything else is refused before a thread starts."},11.,style::secondary_text(cx)).into_any_element());
        v_flex().gap(px(6.)).children(rows).into_any_element()
    }

    fn open_local_trust(&mut self, trust: LocalTrust, window: &mut Window, cx: &mut Context<Self>) {
        self.connectors_panel.setup.trust = Some(trust.clone());
        cx.notify();
        let app = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, cx| {
            let (ok_app, close_app, add_app, cancel_app, trust) = (app.clone(), app.clone(), app.clone(), app.clone(), trust.clone());
            let add_trust = trust.clone();
            let mut body = v_flex().gap(px(10.));
            for (index, line) in trust.lines().into_iter().skip(1).enumerate() {
                let text = text_line(line, 12., style::label_text(cx));
                body = body.child(if index == 1 {
                    text.font_family(cx.theme().mono_font_family.clone()).p(px(8.)).rounded(px(6.)).bg(style::secondary_text(cx).opacity(0.08))
                } else { text });
            }
            dialog.title(format!("Add {} to Connectors?",trust.title)).width(px(440.)).child(body)
                .footer(h_flex().gap(px(8.)).justify_end()
                    .child(tipped_as(div().id("local-trust-cancel"), "Don't add it", Role::GenericContainer)
                        .child(Button::new("local-trust-cancel-button").label("Cancel").on_click(move |_, w, cx| {
                            // Closing a dialog directly does not invoke its on_close callback.
                            cancel_app.update(cx, |t, cx| { t.connectors_panel.setup.trust = None; cx.notify(); }).ok();
                            w.close_dialog(cx);
                        })))
                    .child(tipped_as(div().id("local-trust-add"), "Start a thread that adds it to the gateway with exactly this command", Role::GenericContainer)
                        .child(Button::new("local-trust-add-button").label("Add").on_click(move |_, w, cx| {
                            add_app.update(cx, |t, cx| {
                                t.connectors_panel.setup.trust = None;
                                t.start_connector_thread("connectors.add", add_trust.params.clone(), add_trust.busy_key.clone(), w, cx);
                            }).ok();
                            w.close_dialog(cx);
                        }))))
                .on_ok(move |_,w,cx| { ok_app.update(cx,|t,cx| t.start_connector_thread("connectors.add",trust.params.clone(),trust.busy_key.clone(),w,cx)).ok(); true })
                .on_close(move |_,_,cx| { close_app.update(cx,|t,cx| {t.connectors_panel.setup.trust=None;cx.notify();}).ok(); })
        });
    }

    fn render_outside_agents(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let agents = self
            .connectors_panel
            .setup
            .agents
            .clone()
            .unwrap_or_default();
        let mut rows = vec![small_heading("Agents that use Threadbear", cx).into_any_element()];
        if let Some(reason) = &agents.reason {
            rows.push(text_line(reason.clone(), 12., style::secondary_text(cx)).into_any_element());
        }
        for key in &agents.keys {
            let id = key.id.clone();
            let words = v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(2.))
                .child(text_line(key.name.clone(), 12., style::label_text(cx)))
                .child(text_line(
                    rules::scope_line(key),
                    11.,
                    style::secondary_text(cx),
                ))
                .child(text_line(
                    rules::last_used(key, now_ms()),
                    11.,
                    style::tertiary_label(cx),
                ));
            rows.push(
                h_flex()
                    .gap(px(8.))
                    .items_start()
                    .child(words)
                    .child(push_button(
                        SharedString::from(format!("agent-revoke-{id}")),
                        "Revoke",
                        "End this key: its next call is refused",
                        true,
                        self.connectors_panel
                            .setup
                            .busy
                            .contains(&format!("revoke:{id}")),
                        cx,
                        move |t, _, cx| t.revoke_agent_key(Some(id.clone()), cx),
                    ))
                    .into_any_element(),
            );
        }
        if self.connectors_panel.setup.made.is_some() {
            rows.push(self.render_new_agent_key(cx));
        } else if agents.reason.is_none() && !self.hidden_connectors().contains(rules::HIDDEN_AGENT)
        {
            if self.connectors_panel.setup.agent_form {
                rows.push(self.render_connect_agent(cx));
            } else {
                let group = SharedString::from("connect-agent-offer");
                rows.push(h_flex().group(group.clone()).gap(px(6.)).child(push_button("agent-connect","Connect an agent","Let Grok Bot, Notion or another agent list, read and start threads here through your backend",true,false,cx,|t,_,cx| {t.connectors_panel.setup.agent_form=true;cx.notify();})).child(div().flex_1())
                    .child(self.connector_hide_button(rules::HIDDEN_AGENT.into(),"Hide Connect an agent. The + below brings it back.".into(),group,cx)).into_any_element());
            }
        }
        if agents.keys.len() > 1 {
            rows.push(push_button(
                "agent-revoke-all",
                "Revoke all",
                "End every agent's key now",
                true,
                self.connectors_panel.setup.busy.contains("revoke:all"),
                cx,
                |t, _, cx| t.revoke_agent_key(None, cx),
            ));
        }
        section(rows, cx)
    }

    fn render_connect_agent(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let picked = self.connectors_panel.setup.picked;
        let mut rows = vec![];
        let mut choices = h_flex().gap(px(6.));
        for agent in OutsideAgentApp::ALL {
            choices = choices.child(
                div()
                    .when(agent == picked, |d| {
                        d.border_b_2().border_color(style::accent(cx))
                    })
                    .child(push_button(
                        SharedString::from(format!("agent-pick-{}", agent.name())),
                        agent.name(),
                        format!("Connect {} to Threadbear", agent.name()),
                        true,
                        false,
                        cx,
                        move |t, _, cx| {
                            t.connectors_panel.setup.picked = agent;
                            cx.notify();
                        },
                    )),
            );
        }
        rows.push(choices.into_any_element());
        rows.push(text_line(picked.needs(), 11., style::secondary_text(cx)).into_any_element());
        rows.push(push_button(
            "agent-guide",
            format!("{}'s guide", picked.name()),
            format!("Open {}'s own page on adding an MCP server", picked.name()),
            true,
            false,
            cx,
            move |_, _, cx| {
                crate::open_gate::open_web(picked.guide(), cx);
            },
        ));
        if picked.can_connect() {
            for (write,label,tip) in [(true,"Can start Ask threads and message them","Threads it starts always ask you before any tool runs"),(false,"Can read all my threads and published Prints","Without this it reads only threads it started and no Prints. Only published Prints are shared; archived threads are never shown.")] {
                let checked = if write {self.connectors_panel.setup.write}else{self.connectors_panel.setup.read_all};
                let app = cx.entity().downgrade();
                let toggle = Checkbox::new(if write {"agent-write"}else{"agent-read-all"}).checked(checked).label(label).small().on_click(move|on:&bool,_,cx|{app.update(cx,|t,cx|{if write{t.connectors_panel.setup.write = *on;}else{t.connectors_panel.setup.read_all = *on;}cx.notify();}).ok();});
                rows.push(tipped_as(div().id(if write {"agent-write-holder"}else{"agent-read-holder"}),tip,Role::GenericContainer).child(toggle).into_any_element());
            }
        }
        let mut buttons = h_flex().gap(px(6.));
        if picked.can_connect() {
            buttons = buttons.child(push_button(
                "agent-make",
                "Make key",
                format!("Make a key for {}, shown once", picked.name()),
                true,
                self.connectors_panel.setup.busy.contains("make-key"),
                cx,
                |t, _, cx| t.make_agent_key(cx),
            ));
        }
        buttons = buttons.child(push_button(
            "agent-cancel",
            if picked.can_connect() {
                "Cancel"
            } else {
                "Close"
            },
            "Put the agent form away",
            true,
            false,
            cx,
            |t, _, cx| {
                t.connectors_panel.setup.agent_form = false;
                cx.notify();
            },
        ));
        rows.push(buttons.into_any_element());
        v_flex().gap(px(8.)).children(rows).into_any_element()
    }

    #[expect(
        clippy::unwrap_used,
        reason = "only drawn while what it unwraps is set, just above or by its caller"
    )]
    fn render_new_agent_key(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let made = self.connectors_panel.setup.made.as_ref().unwrap();
        let agent = OutsideAgentApp::for_key(&made.key.name);
        let fields = agent.form(&made.url, &made.secret);
        let mut rows = vec![
            text_line(
                "Copy these now. The key can't be copied again once this panel closes.",
                12.,
                style::label_text(cx),
            )
            .into_any_element(),
            text_line(agent.intro(), 11., style::secondary_text(cx)).into_any_element(),
        ];
        for field in fields {
            let mut row = h_flex()
                .gap(px(6.))
                .items_center()
                .child(
                    text_line(field.label, 11., style::secondary_text(cx))
                        .w(px(84.))
                        .flex_none(),
                )
                .child(
                    tipped_as(
                        div()
                            .id(SharedString::from(format!("agent-field-{}", field.label)))
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.))
                            .truncate(),
                        field.tip,
                        Role::GenericContainer,
                    )
                    .child(field.shown),
                );
            if field.copies {
                // Read the secret only when pressed. Cached click handlers must not
                // retain it after Done/close; a stale handler becomes a no-op.
                let label = field.label;
                row = row.child(icon_button(
                    SharedString::from(format!("agent-copy-{label}")),
                    "doc.on.doc",
                    field.tip,
                    false,
                    cx,
                    move |t, _, cx| {
                        if let Some(made) = t.connectors_panel.setup.made.as_ref() {
                            if let Some(f) = OutsideAgentApp::for_key(&made.key.name)
                                .form(&made.url, &made.secret)
                                .into_iter()
                                .find(|f| f.label == label)
                            {
                                cx.write_to_clipboard(ClipboardItem::new_string(f.value));
                            }
                        }
                    },
                ));
            }
            rows.push(row.into_any_element());
        }
        rows.push(text_line(agent.outro(), 11., style::secondary_text(cx)).into_any_element());
        rows.push(push_button(
            "agent-key-done",
            "Done",
            "Put the key away",
            true,
            false,
            cx,
            |t, _, cx| {
                t.connectors_panel.setup.made = None;
                cx.notify();
            },
        ));
        v_flex()
            .gap(px(8.))
            .p(px(10.))
            .rounded(px(6.))
            .bg(style::secondary_text(cx).opacity(0.08))
            .children(rows)
            .into_any_element()
    }

    pub(crate) fn connector_setup_lines(&self) -> Vec<String> {
        let mut lines = vec![];
        let hidden = self.hidden_connectors();
        if let Some(s) = self.visible_connectors() {
            if s.can_set_up {
                lines.push("Set up".into());
            }
            if s.state == "ok" {
                if self
                    .connectors_panel
                    .status
                    .as_ref()
                    .is_some_and(|s| !s.catalog.is_empty() || s.can_add_local)
                {
                    lines.push("Add a service".into());
                }
                for e in s.catalog {
                    lines.push(if e.is_local() {
                        format!("{}: On this Mac [Add...]", e.title)
                    } else if e.client_setup.is_some() {
                        format!("{} [{}]", e.title, rules::create_app_title(&e.title))
                    } else {
                        format!("{} [Add]", e.title)
                    });
                }
            }
        }
        let state = &self.connectors_panel.setup;
        if rules::show_agents(state.agents.as_ref(), &hidden) {
            lines.push("Agents that use Threadbear".into());
            if let Some(a) = &state.agents {
                lines.extend(a.reason.clone());
                for k in &a.keys {
                    lines.push(format!("{}: {}", k.name, rules::scope_line(k)));
                    lines.push(rules::last_used(k, now_ms()));
                }
                if state.made.is_none()
                    && a.reason.is_none()
                    && !hidden.contains(rules::HIDDEN_AGENT)
                {
                    if state.agent_form {
                        lines.push(state.picked.needs().into());
                        if state.picked.can_connect() {
                            lines.push("Make key".into());
                        }
                    } else {
                        lines.push("Connect an agent".into());
                    }
                }
            }
            if let Some(made) = &state.made {
                lines.extend(
                    OutsideAgentApp::for_key(&made.key.name)
                        .form(&made.url, &made.secret)
                        .into_iter()
                        .map(|f| {
                            format!(
                                "{}: {}{}",
                                f.label,
                                f.shown,
                                if f.copies { " [Copy]" } else { "" }
                            )
                        }),
                );
            }
        }
        if let Some(trust) = &state.trust {
            lines.extend(trust.lines());
        }
        if let Some(form) = &state.client {
            lines.push(format!(
                "{} keys: Client ID, Client Secret [Save] [Cancel]",
                form.id
            ));
        }
        let count = rules::hidden_items(
            &hidden,
            self.connectors_panel.status.as_ref(),
            state.agents.as_ref(),
        )
        .len();
        if count > 0 {
            lines.push(format!("+ {count} hidden"));
        }
        lines
    }
}

fn section(rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    v_flex()
        .gap(px(10.))
        .border_t_1()
        .border_color(style::hairline(cx))
        .pt(px(12.))
        .children(rows)
        .into_any_element()
}
fn input_row(
    id: &'static str,
    label: &'static str,
    input: &Entity<InputState>,
    cx: &App,
) -> AnyElement {
    v_flex()
        .gap(px(3.))
        .child(text_line(label, 11., style::secondary_text(cx)))
        .child(
            tipped_as(div().id(id), label, Role::GenericContainer).child(Input::new(input).small()),
        )
        .into_any_element()
}

/// Scripts scroll a long form to its control before clicking, as a person does.
#[cfg(feature = "automation")]
pub(crate) fn scroll_into_view(app: &Entity<Threadbear>, bounds: Bounds<Pixels>, cx: &App) -> bool {
    let scroll = &app.read(cx).connectors_panel.scroll;
    let (pane, offset, most) = (scroll.bounds(), scroll.offset(), scroll.max_offset());
    // Dialog controls are outside the scrolling panel.
    if bounds.left() < pane.left() || bounds.right() > pane.right() {
        return false;
    }
    let margin = gpui_kit::px(12.);
    let y = if bounds.bottom() > pane.bottom() {
        offset.y - (bounds.bottom() - pane.bottom()) - margin
    } else if bounds.top() < pane.top() {
        offset.y + (pane.top() - bounds.top()) + margin
    } else {
        return false;
    };
    scroll.set_offset(point(
        offset.x,
        y.max(gpui_kit::px(0.) - most.y).min(gpui_kit::px(0.)),
    ));
    true
}
