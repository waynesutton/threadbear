//! The runner's shared skill store, over the current thread.
use crate::{app::Threadbear, call, style};
use crate::style::px;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, StyledExt as _};
use gpui_kit::component::input::{Input, InputState, InputEvent, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};

#[derive(Default)]
pub(crate) struct SkillsPage {
    pub open: bool,
    pub supported: bool,
    pub enabled: bool,
    list: Vec<Value>,
    suggested: Vec<Value>,
    busy: bool,
    error: Option<String>,
    link: Option<Entity<InputState>>,
    draft: Option<SkillDraft>,
    subscriptions: Vec<Subscription>,
    scroll: ScrollHandle,
}
struct SkillDraft {
    name: Entity<InputState>,
    description: Entity<InputState>,
    text: Entity<TextareaState>,
    replacing: Option<String>,
}
fn words(v: &Value, key: &str) -> String { v[key].as_str().unwrap_or_default().to_owned() }
fn button(id: impl Into<SharedString>, title: String, busy: bool) -> impl StatefulInteractiveElement + ParentElement + IntoElement {
    div().id(ElementId::Name(id.into())).test_support().role(Role::Button).aria_label(title.clone())
        .px(px(10.)).py(px(6.)).rounded_md().cursor_pointer().opacity(if busy {0.45} else {1.})
        .tip(title.clone()).child(title)
}
impl SkillsPage {
    pub(crate) fn receive(&mut self, value: &Value) {
        self.supported = true;
        if let Some(list) = value["skills"].as_array() { self.list = list.clone(); }
        if let Some(list) = value["suggested"].as_array() { self.suggested = list.clone(); }
        if let Some(enabled) = value["enabled"].as_bool() { self.enabled = enabled; }
        self.suggested.retain(|s| !self.list.iter().any(|k| k["name"] == s["name"]));
    }
    #[cfg(feature="automation")]
    pub(crate) fn dump(&self, cx: &App) -> Value {
        json!({"open":self.open,"list":self.list,"enabled":self.enabled,"busy":self.busy,"error":self.error,
            "text":self.draft.as_ref().map(|d| d.text.read(cx).value().to_string())})
    }
}
impl Threadbear {
    pub(crate) fn refresh_skill_commands(&mut self, cx: &mut Context<Self>) {
        self.slash.commands.clear();
        if let Some(id)=self.selected.clone() {self.load_slash_commands(id,cx);} else {self.load_new_thread_slash_commands(cx);}
    }
    pub(crate) fn load_skills(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this,cx| {
            let result = call(client,"skills.list",json!({})).await;
            this.update(cx,|this,cx| {
                match result {
                    Ok(v) => this.skills.receive(&v),
                    Err(e) if e.code == -32601 => this.skills.supported = false,
                    Err(e) => this.skills.error = Some(e.message),
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    pub(crate) fn show_skills(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.skills.supported && !self.skills.enabled {return;}
        self.hide_settings_page(cx);
        self.notes.open = false;
        self.skills.open = true;
        if self.skills.link.is_none() {
            self.skills.link = Some(cx.new(|cx| InputState::new(window,cx).placeholder("A GitHub link to a skill or repository")));
        }
        window.focus(&self.sidebar.focus,cx);
        crate::web_panels::keyboard_to_app(window);
        self.load_skills(cx);
        cx.notify();
    }
    pub(crate) fn close_skills(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.skills.open = false;
        self.close_settings_page(window,cx);
        cx.notify();
    }
    pub(crate) fn skill_change(&mut self, method: &'static str, params: Value, cx: &mut Context<Self>) {
        if self.skills.busy { return; }
        self.skills.busy = true;
        self.skills.error = None;
        let client = self.client.clone();
        cx.notify();
        cx.spawn(async move |this,cx| {
            let result = call(client,method,params).await;
            this.update(cx,|this,cx| {
                this.skills.busy = false;
                match result {
                    Ok(v) => {
                        this.skills.receive(&v);
                        this.refresh_skill_commands(cx);
                        if method == "skills.save" { this.skills.draft = None; }
                        if !this.skills.enabled { this.skills.open = false; }
                    }
                    Err(e) => this.skills.error = Some(e.message),
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn skill_draft(&mut self, name: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.skills.busy || (name.is_none() && self.skills.draft.is_some()) { return; }
        let replacing = name.clone();
        let input = cx.new(|cx| { let mut s = InputState::new(window,cx).placeholder("name, like brand-voice"); s.set_value(name.unwrap_or_default(),window,cx); s });
        let description = cx.new(|cx| InputState::new(window,cx).placeholder("When should agents use it?"));
        let text = cx.new(|cx| TextareaState::new(window,cx).placeholder("The skill's instructions, or a whole SKILL.md"));
        window.focus(&input.read(cx).focus_handle(cx),cx);
        crate::web_panels::keyboard_to_app(window);
        self.skills.subscriptions.clear();
        for field in [&input,&description] {
            self.skills.subscriptions.push(cx.subscribe(field,|this,_,event:&InputEvent,cx| {
                if matches!(event,InputEvent::PressEnter{..}) {this.save_skill(cx);}
            }));
        }
        self.skills.draft = Some(SkillDraft{name:input,description,text,replacing:replacing.clone()});
        self.skills.error = None;
        if let Some(name) = replacing {
            self.skills.busy = true;
            let client = self.client.clone();
            cx.spawn_in(window,async move |this,cx| {
                let result = call(client,"skills.read",json!({"name":name})).await;
                this.update_in(cx,|this,window,cx| {
                    this.skills.busy = false;
                    match result {
                        Ok(v) => if let Some(d) = &this.skills.draft { if d.replacing.as_ref() == Some(&name) { d.text.update(cx,|s,cx|s.set_value(words(&v,"text"),window,cx)); } },
                        Err(e) => this.skills.error = Some(e.message),
                    }
                    cx.notify();
                }).ok();
            }).detach();
        }
        cx.notify();
    }
    fn save_skill(&mut self,cx:&mut Context<Self>) {
        let Some(d) = &self.skills.draft else { return; };
        let name = d.name.read(cx).value().trim().trim_start_matches('/').to_lowercase();
        let text = d.text.read(cx).value().to_string();
        if name.is_empty() || text.trim().is_empty() { self.skills.error = Some("Give the skill a name and instructions.".into()); cx.notify(); return; }
        self.skill_change("skills.save",json!({"name":name,"text":text,"description":d.description.read(cx).value().to_string(),"replacing":d.replacing}),cx);
    }
    pub(crate) fn render_skills(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.skills.busy;
        let mut content = v_flex().text_size(px(13.)).font_weight(FontWeight::NORMAL).line_height(px(20.)).gap(px(18.)).p(px(24.)).w_full().max_w(px(720.))
            .child("Skills give every thread's agent extra know-how, on every login, Claude and Codex alike. Always on lets agents use a skill whenever it fits. Otherwise, type its slash command.");
        if busy { content = content.child("Working…"); }
        if let Some(error) = &self.skills.error { content = content.child(div().text_color(cx.theme().foreground).child(error.clone())); }
        if let Some(d) = &self.skills.draft {
            content = content.child(v_flex().gap(px(10.)).p(px(14.)).border_1().border_color(cx.theme().border).rounded_lg()
                .child(if let Some(name)=&d.replacing {format!("Edit {name}")} else {"New skill".into()})
                .child(Input::new(&d.name)).child(Input::new(&d.description))
                .child(Textarea::new(&d.text).h(px(200.)))
                .child(h_flex().gap(px(10.)).child(button("skill-cancel","Cancel skill".into(),busy).on_click(cx.listener(|this,_,_,cx|{if !this.skills.busy {this.skills.draft=None;cx.notify();}})))
                    .child(button("skill-save","Save skill".into(),busy).on_click(cx.listener(|this,_,_,cx|this.save_skill(cx))))));
        }
        if let Some(link) = &self.skills.link {
            content = content.child(v_flex().gap(px(8.)).child("Import a skill")
                .child(Input::new(link)).child(button("skill-import","Import skill".into(),busy).on_click(cx.listener(|this,_,_,cx|{
                    let url=this.skills.link.as_ref().map(|l|l.read(cx).value().to_string()).unwrap_or_default();
                    this.skill_change("skills.add",json!({"url":url.trim()}),cx);
                }))));
        }
        content = content.child("Installed skills");
        for skill in self.skills.list.clone() {
            let name=words(&skill,"name"); let always=skill["always"].as_bool().unwrap_or(false);
            let toggle=name.clone();let remove=name.clone();let edit=name.clone();let latest=name.clone();
            let mut row=v_flex().id(ElementId::Name(format!("skill-{name}").into())).role(Role::ListItem).gap(px(6.)).p(px(12.)).rounded_lg().bg(style::primary_opacity(cx,0.04))
                .child(format!("/{name}")).child(words(&skill,"description"));
            let mut actions=h_flex().flex_wrap().gap(px(6.))
                .child(button(format!("skill-always-{name}"),format!("Always on {name}: {}",if always {"On"} else {"Off"}),busy).on_click(cx.listener(move |this,_,_,cx|this.skill_change("skills.update",json!({"name":toggle,"always":!always}),cx))));
            if skill["source"]["kind"] == "custom" {
                actions=actions.child(button(format!("skill-edit-{name}"),format!("Edit {name}"),busy).on_click(cx.listener(move |this,_,window,cx|this.skill_draft(Some(edit.clone()),window,cx))));
            }
            if skill["source"]["kind"] == "github" {
                row=row.child(words(&skill["source"],"repo"));
                actions=actions.child(button(format!("skill-update-{name}"),format!("Update {name}"),busy).on_click(cx.listener(move |this,_,_,cx|this.skill_change("skills.update",json!({"name":latest,"latest":true}),cx))));
            }
            if let Some(url)=skill["source"]["url"].as_str().map(str::to_owned) {
                actions=actions.child(button(format!("skill-web-{name}"),format!("View {name} on GitHub"),busy).on_click(move |_,_,cx|{crate::open_gate::open_web(&url,cx);}));
            }
            let path=std::path::Path::new(&words(&skill,"dir")).join("SKILL.md").to_string_lossy().into_owned();
            actions=actions.child(button(format!("skill-file-{name}"),format!("Show {name} in folder"),busy).on_click(move |_,_,cx|crate::open_gate::reveal(&path,crate::open_gate::Origin::ThisMac,cx)));
            row=row.child(format!("{} · {}",words(&skill["source"],"kind"),words(&skill,"commit").chars().take(12).collect::<String>()));
            actions=actions.child(button(format!("skill-remove-{name}"),format!("Remove {name}"),busy).on_click(cx.listener(move |this,_,_,cx|this.skill_change("skills.remove",json!({"name":remove}),cx))));
            content=content.child(row.child(actions));
        }
        for suggested in self.skills.suggested.clone() {
            let id=words(&suggested,"id"); let name=words(&suggested,"name");
            content=content.child(v_flex().gap(px(6.)).child(format!("Suggested: {name}")).child(words(&suggested,"description"))
                .child(button(format!("skill-add-{id}"),format!("Add {name}"),busy).on_click(cx.listener(move |this,_,_,cx|this.skill_change("skills.add",json!({"suggested":id}),cx)))));
        }
        v_flex().id("skills-page").size_full().bg(cx.theme().background)
            .on_key_down(cx.listener(|this,event:&KeyDownEvent,window,cx|{
                if event.keystroke.key=="escape" { this.close_skills(window,cx);cx.stop_propagation(); }
                if event.keystroke.key=="enter" && event.keystroke.modifiers.platform {this.save_skill(cx);cx.stop_propagation();}
            }))
            .child(h_flex().h(px(52.)).flex_none().px(px(20.)).gap(px(10.)).child(div().flex_1().font_semibold().child("Skills"))
                .child(button("skills-new","New Skill".into(),busy).on_click(cx.listener(|this,_,window,cx|this.skill_draft(None,window,cx))))
                .child(button("skills-close","Close Skills".into(),false).on_click(cx.listener(|this,_,window,cx|this.close_skills(window,cx)))))
            .child(div().id("skills-scroll").track_scroll(&self.skills.scroll).flex_1().min_h_0().overflow_y_scroll().child(content)).into_any_element()
    }
    #[cfg(feature="automation")]
    pub(crate) fn skills_automation(&mut self,cmd:&Value,window:&mut Window,cx:&mut Context<Self>)->Result<Value,String> {
        if let Some(by)=cmd["scroll"].as_f64() {
            let offset=self.skills.scroll.offset();
            self.skills.scroll.set_offset(point(offset.x,offset.y-gpui_kit::px(by as f32)));
            cx.notify();return Ok(json!("ok"));
        }
        if let Some(url)=cmd["url"].as_str() {
            let link=self.skills.link.as_ref().ok_or("Skills isn't open")?;
            link.update(cx,|s,cx|s.set_value(url,window,cx));
            cx.notify();return Ok(json!("ok"));
        }
        let d=self.skills.draft.as_ref().ok_or("no skill editor")?;
        if let Some(name)=cmd["name"].as_str(){d.name.update(cx,|s,cx|s.set_value(name,window,cx));}
        if let Some(text)=cmd["text"].as_str(){d.text.update(cx,|s,cx|s.set_value(text,window,cx));}
        cx.notify();Ok(json!("ok"))
    }
}
