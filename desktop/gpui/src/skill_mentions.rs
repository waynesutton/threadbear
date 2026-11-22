// Skills listed by this thread's CLI remain ordinary editable words, coloured
// in the box and linked to their file in sent messages.
use crate::{composer::Composer, style, thread::ThreadPane};
use gpui_kit::component::input::TextDecoration;
use gpui_kit::*;
use threadbear_core::{skill_mentions, slash::SlashCommand};

impl Composer {
    pub(crate) fn mention_commands(&self, cx: &App) -> Vec<SlashCommand> {
        let key = self.palette_list_key(cx);
        self.app
            .upgrade()
            .and_then(|app| app.read(cx).slash.commands.get(&key).cloned())
            .unwrap_or_default()
    }

    pub(crate) fn skill_marks(&self, cx: &App) -> Vec<TextDecoration> {
        let text = self.input.read(cx).value().to_string();
        let commands = self.mention_commands(cx);
        skill_mentions::find(&text, &commands)
            .into_iter()
            .map(|m| {
                TextDecoration::new(
                    m.range,
                    HighlightStyle {
                        color: Some(style::accent(cx)),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    #[cfg(feature = "automation")]
    pub(crate) fn skill_mentions_dump(&self, cx: &App) -> serde_json::Value {
        let input = self.input.read(cx);
        let text = input.value().to_string();
        let commands = self.mention_commands(cx);
        serde_json::json!(skill_mentions::find(&text, &commands)
            .into_iter()
            .filter(|m| input
                .text_marks()
                .iter()
                .any(|mark| mark.range == m.range && mark.style.color == Some(style::accent(cx))))
            .map(|m| text[m.range].to_owned())
            .collect::<Vec<_>>())
    }

    /// Fetch before the first slash, once for each thread or draft login/folder.
    pub(crate) fn preload_mentions(&mut self, cx: &mut Context<Self>) {
        let key = self.palette_list_key(cx);
        if self.mentions_asked.as_deref() == Some(key.as_str()) {
            return;
        }
        self.mentions_asked = Some(key);
        let app = self.app.clone();
        let thread = self.thread_id.clone();
        cx.defer(move |cx| {
            app.update(cx, |app, cx| match thread {
                Some(id) => app.load_slash_commands(id, cx),
                None => app.load_new_thread_slash_commands(cx),
            })
            .ok();
        });
    }
}

impl ThreadPane {
    pub(crate) fn skill_words(
        &mut self,
        id: &str,
        text: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let mentions = skill_mentions::find(text, &self.mention_commands(cx));
        let linked = skill_mentions::linked_markdown_text(text, &mentions);
        let source = threadbear_core::user_markdown::prepare(&linked)
            .or_else(|| (!mentions.is_empty()).then_some(linked))?;
        let source = crate::rows::markdown::link_code_paths(&source);
        let doc = self.markdown_state(&format!("user-skills-{id}"), &source, cx);
        let style = self
            .reading_style(cx)
            .with_foreground(style::label_text(cx))
            .with_paragraph_gap(gpui_kit::rems(0.));
        Some(
            self.linked_text(&doc, id, cx)
                .style(style)
                .into_any_element(),
        )
    }

    pub(crate) fn mention_commands(&self, cx: &App) -> Vec<SlashCommand> {
        self.app
            .upgrade()
            .and_then(|app| app.read(cx).slash.commands.get(&self.summary.id).cloned())
            .unwrap_or_default()
    }

    pub(crate) fn open_skill_link(&mut self, name: &str, cx: &mut Context<Self>) {
        // Scripted links can arrive inside an App update. Resolve the catalog
        // and show the file after that borrow, as a real link click does.
        let pane = cx.weak_entity();
        let name = name.to_owned();
        cx.defer(move |cx| {
            pane.update(cx, |this, cx| {
                let path = this
                    .mention_commands(cx)
                    .into_iter()
                    .find(|c| c.is_skill() && c.name.eq_ignore_ascii_case(&name))
                    .and_then(|c| c.path);
                if let Some(path) = path {
                    this.preview_in_pane(&path, None, cx);
                } else {
                    this.show_error(
                        "This runner hasn't listed a file for that skill.".to_owned(),
                        cx,
                    );
                }
            })
            .ok();
        });
    }
}
