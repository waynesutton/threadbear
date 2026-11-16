//! The sidebar mascot opens an ordinary local thread. Voice stays separate.
use crate::{app::Threadbear, style};
use crate::style::px;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::menu::{ContextMenuExt as _,PopupMenuItem};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;
use threadbear_core::bear_chat;

fn user_home() -> PathBuf {
    // Scripted apps must never prepare folders in the person's real home.
    if std::env::var_os("THREADBEAR_AUTOMATION").is_some() {
        threadbear_client::config::RunnerConfig::from_env().home.join("bear-home")
    } else { PathBuf::from(crate::files::home_dir()) }
}
impl Threadbear {
    pub(crate) fn render_bear_chat(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let thread=bear_chat::newest(&user_home(),self.threads.values().map(|t|t.as_ref()));
        let selected=thread.is_some_and(|t|self.selected.as_ref()==Some(&t.id));
        let thinking=self.bear_preparing || thread.is_some_and(|t|matches!(t.status.as_str(),"running"|"starting"));
        let weak=cx.entity().downgrade();
        let glyph=if thinking {Spinner::new().small().into_any_element()} else {div().text_size(px(22.)).child("🐻").into_any_element()};
        div().id("sidebar-bear-chat").test_support().role(Role::Button).aria_label("Chat with the bear")
            .w(px(30.)).flex_none().flex().items_center().justify_center().rounded(px(6.)).cursor_pointer()
            .when(selected,|s|s.bg(style::primary_opacity(cx,0.08))).hover(|s|s.bg(style::primary_opacity(cx,0.08))).tip("Chat with the bear")
            .on_click(cx.listener(|this,_,window,cx|this.prepare_bear(false,window,cx))).child(glyph)
            .context_menu(move |menu,_,_| {
                let open=weak.clone();let folder=weak.clone();
                menu.item(PopupMenuItem::new("Open Chat").on_click(move |_,window,cx|{open.update(cx,|this,cx|this.prepare_bear(false,window,cx)).ok();}))
                    .item(PopupMenuItem::new("Show the Bear's Folder").on_click(move |_,window,cx|{folder.update(cx,|this,cx|this.prepare_bear(true,window,cx)).ok();}))
            }).into_any_element()
    }
    fn prepare_bear(&mut self, reveal:bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.bear_preparing {return;}
        self.bear_preparing=true;
        cx.notify();
        let home=user_home();
        let prepare=cx.background_executor().spawn(async move {bear_chat::prepare(&home)});
        cx.spawn_in(window,async move |this,cx| {
            let result=prepare.await;
            this.update_in(cx,|this,window,cx| {
                this.bear_preparing=false;
                match result {
                    Ok(folder)=>{
                        if reveal {
                            crate::open_gate::reveal(&folder.join("CLAUDE.md").to_string_lossy(),crate::open_gate::Origin::ThisMac,cx);
                            cx.notify();return;
                        }
                        let thread=bear_chat::newest(&user_home(),this.threads.values().map(|t|t.as_ref())).map(|t|t.id.clone());
                        if let Some(id)=thread {this.open(id,window,cx);}
                        else {
                            this.new_thread(cx);
                            this.choose_new_thread_runner(None,true,cx);
                            this.change_new_draft(&serde_json::json!({"cwd":folder.to_string_lossy()}),cx);
                        }
                    }
                    Err(e)=>this.show_error(format!("The bear's folder couldn't be prepared: {e}"),cx),
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    #[cfg(feature="automation")]
    pub(crate) fn bear_chat_dump(&self)->serde_json::Value {
        serde_json::json!({"folder":bear_chat::folder(&user_home()),"preparing":self.bear_preparing,"thread":bear_chat::newest(&user_home(),self.threads.values().map(|t|t.as_ref())).map(|t|&t.id)})
    }
}
