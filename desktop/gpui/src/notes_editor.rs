//! Markdown behaviour in the private Notes editor. All edits go through its
//! normal replacement path, so undo and autosave work for task clicks too.
use crate::{app::Threadbear, style};
use crate::style::px;
use gpui_kit::component::{ActiveTheme as _, input::{EditorState, Enter, Undo, TextDecoration}};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::notes_edit::{self, Kind, Newline};
use serde_json::{json, Value};
#[cfg(feature="automation")]
use gpui_kit::test::{TestWindowExt as _, TestEventExt as _};

impl Threadbear {
    pub(crate) fn refresh_note_format(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.notes.editor.clone() else { return };
        let text = editor.read(cx).value().to_string();
        let spans = notes_edit::spans(&text);
        let mut scales = std::collections::BTreeMap::new();
        let foreground = cx.theme().foreground;
        let secondary = style::secondary_text(cx);
        let accent = style::accent(cx);
        // Notes owns its small Markdown palette, including neutral headings.
        // Flatten overlaps before handing decorations to the generic editor.
        let mut runs=Vec::new();
        for span in spans {
            let highlight = match span.kind {
                Kind::Heading(level) => {
                    let row=text[..span.range.start].bytes().filter(|b|*b==b'\n').count();
                    scales.insert(row,match level {1=>1.6,2=>1.35,3=>1.15,_=>1.});
                    HighlightStyle {color:Some(foreground),font_weight:Some(FontWeight::BOLD),..Default::default()}
                }
                Kind::Task(_) => HighlightStyle {color:Some(accent),font_weight:Some(FontWeight::BOLD),..Default::default()},
                Kind::Done => HighlightStyle {color:Some(secondary),strikethrough:Some(StrikethroughStyle {thickness:px(1.),color:Some(secondary)}),..Default::default()},
                Kind::Bold => HighlightStyle {font_weight:Some(FontWeight::BOLD),..Default::default()},
                Kind::Italic => HighlightStyle {font_style:Some(FontStyle::Italic),..Default::default()},
                Kind::Code => HighlightStyle {color:Some(rgb(0xd65c9d).into()),background_color:Some(style::primary_opacity(cx,0.04)),..Default::default()},
                Kind::CodeBlock | Kind::Quote | Kind::Mark => HighlightStyle {color:Some(secondary),..Default::default()},
                Kind::Link => HighlightStyle {color:Some(accent),..Default::default()},
            };
            runs.push((span.range,highlight));
        }
        let mut cuts=vec![0,text.len()];
        for (range,_) in &runs {cuts.push(range.start);cuts.push(range.end);}
        cuts.sort_unstable(); cuts.dedup();
        let decorations=cuts.windows(2).filter_map(|cut| {
            if cut[0]==cut[1] {return None}
            let mut style=HighlightStyle::default();
            for (range,highlight) in &runs {if range.start<=cut[0] && range.end>=cut[1] {style=style.highlight(*highlight);}}
            Some(TextDecoration::new(cut[0]..cut[1],style))
        }).collect();
        editor.update(cx, |s,cx|s.set_line_font_scales(scales,cx));
        match &self.notes.decorations {
            Some(collection) => collection.set(decorations,cx),
            None => self.notes.decorations=Some(editor.update(cx,|s,cx|s.create_decorations_collection(decorations,cx))),
        }
    }
}

pub(crate) fn return_in_note(editor: &Entity<EditorState>, window: &mut Window, cx: &mut App) -> bool {
    let (text,selection) = {let s=editor.read(cx); (s.value().to_string(),s.selected_range())};
    if !selection.is_empty() {return false}
    let (range,replacement) = match notes_edit::newline(&text,selection.start) {
        Newline::Plain=>return false,
        Newline::Continue(prefix)=>(selection,prefix),
        Newline::End(range)=>(range,String::new()),
    };
    editor.update(cx,|s,cx| {s.set_selected_range(range,cx);s.replace(replacement,window,cx);});
    true
}

/// A capture listener leaves ordinary presses to the editor, but a press on
/// a task marker replaces its middle character before selection takes it.
pub(crate) fn task_clicks(editor: Entity<EditorState>) -> impl IntoElement {
    canvas(|_,_,_|(), move |bounds,_,window,_| {
        let editor=editor.clone();
        window.on_mouse_event(move |event: &MouseDownEvent,phase,window,cx| {
            if phase!=DispatchPhase::Capture || event.button!=MouseButton::Left || !bounds.contains(&event.position) {return}
            let at=editor.read(cx).offset_for_point(event.position);
            let text=editor.read(cx).value().to_string();
            if let Some((range,replacement))=notes_edit::toggle(&text,at) {
                editor.update(cx,|s,cx| {s.set_selected_range(range,cx);s.replace(replacement,window,cx);});
                window.focus(&editor.read(cx).focus_handle(cx),cx);
                crate::web_panels::keyboard_to_app(window);
                cx.stop_propagation();
            }
        });
    }).absolute().top_0().left_0().size_full()
}

#[cfg(feature="automation")]
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value,String> {
    app.update(cx,|t,cx|t.notes_command(cmd,window,cx))?;
    let editor=app.read(cx).notes.editor.clone();
    if let Some(at)=cmd["returnAt"].as_u64().or(cmd["toggleAt"].as_u64()) {
        let editor=editor.as_ref().ok_or("no note is open")?;
        editor.update(cx,|s,cx|s.set_selected_range(at as usize..at as usize,cx));
        window.focus(&editor.read(cx).focus_handle(cx),cx);
                crate::web_panels::keyboard_to_app(window);
        window.render_frame(cx);
        if cmd.get("toggleAt").is_some() {
            let (caret,_) = editor.read(cx).cursor_layout().ok_or("task is outside the editor")?;
            let at=caret.center()+point(px(2.),px(0.));
            window.pointer_move(at,None,cx);
            window.pointer_down(at,MouseButton::Left,cx);
            window.pointer_up(at,MouseButton::Left,cx);
        } else {window.dispatch_action(Box::new(Enter {secondary:false,shift:false}),cx);}
    }
    if cmd["undo"]==true {
        let editor=editor.as_ref().ok_or("no note is open")?;
        window.focus(&editor.read(cx).focus_handle(cx),cx);
                crate::web_panels::keyboard_to_app(window);
        window.dispatch_action(Box::new(Undo),cx);
    }
    Ok(json!("ok"))
}
