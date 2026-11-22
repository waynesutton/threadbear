// A Print in a reply (rich.print): a ```print <id> fence (```page before the
// rename to Prints) drawn as the Print's card, "Preview on top" as Wayne
// picked on 10 Nov 2026: the top of its newest version as a picture, over a
// footer with its kind's icon, title, version and age, and Open, Refresh and
// a ⋯ menu. A port of the Swift app's PageCardCell and PageCards. It follows
// `pages.updated`, so every card naming a Print shows its newest version,
// marked "New version" for 15 minutes when it's past the first. Open puts it
// beside the thread, in the side pane, where it follows new versions too.
//
// With Prints off, or for a block still streaming in, the fence stays the
// kit's code, as it was. The picture is drawn as a reply's page is
// (rich_boxes.rs), so it needs the web runtime; without it the card says so
// and works otherwise.
use crate::app::Threadbear;
use crate::prints::PrintAction;
use crate::rich_boxes::Thumb;
use crate::style::{self, px, size};
use crate::tips::Tip as _;
use crate::{call, icons, now_ms};
use gpui_kit::base::text::{MarkdownNode, MarkdownParseContext, MarkdownPlugin};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use threadbear_core::prints::{self as core, Page};
use threadbear_core::rich_block::{RichBlock, RichKind};

const NODE: &str = "print-card";
/// PageCardCell's measures, in points at 100%.
pub(crate) const PREVIEW: f32 = 170.;
pub(crate) const FOOTER: f32 = 56.;
const PAD: f32 = 12.;
const RADIUS: f32 = 10.;

pub(crate) const OPEN_TIP: &str = "Open this print beside the thread";
pub(crate) const REFRESH_TIP: &str = "Ask the thread that made it to make it again the same way";
pub(crate) const MORE_TIP: &str = "Edit with Agent, edit its text, download, versions or delete";

/// What the cards share: the app, and each Print's newest page as a block
/// to draw (read from its file once per version).
pub(crate) struct CardStore {
    app: Option<WeakEntity<Threadbear>>,
    blocks: HashMap<String, (u32, RichBlock)>,
    asking: HashSet<String>,
}

struct Shared(Entity<CardStore>);
impl Global for Shared {}

pub(crate) fn init(cx: &mut App) {
    let store = cx.new(|_| CardStore {
        app: None,
        blocks: HashMap::new(),
        asking: HashSet::new(),
    });
    cx.set_global(Shared(store));
}

fn store(cx: &App) -> Option<Entity<CardStore>> {
    cx.try_global::<Shared>().map(|s| s.0.clone())
}

/// The Prints changed (a new list, a call going out or back): the cards
/// draw again.
pub(crate) fn changed(cx: &mut Context<Threadbear>) {
    let app = cx.entity().downgrade();
    if let Some(store) = store(cx) {
        store.update(cx, |s, cx| {
            s.app = Some(app);
            cx.notify();
        });
    }
}

/// A pane's rows draw again as the cards change.
pub(crate) fn watch(cx: &mut Context<crate::thread::ThreadPane>) -> Option<Subscription> {
    let store = store(cx)?;
    Some(cx.observe(&store, |_, _, cx| cx.notify()))
}

impl CardStore {
    /// The block that draws the Print's newest page, or None while its file
    /// is being read (the cards draw again when it comes).
    fn block(&mut self, page: &Page, cx: &mut Context<Self>) -> Option<RichBlock> {
        if let Some((v, block)) = self.blocks.get(&page.id) {
            if *v == page.version {
                return Some(block.clone());
            }
        }
        let app = self.app.as_ref()?.upgrade()?;
        if !self.asking.insert(page.id.clone()) {
            return None;
        }
        let (client, id) = (app.read(cx).client.clone(), page.id.clone());
        cx.spawn(async move |this, cx| {
            let got = call(client, "pages.get", json!({ "pageId": id })).await;
            let found = match got {
                Ok(v) => {
                    let version = v["page"]["version"].as_u64().unwrap_or(0) as u32;
                    match v["file"].as_str().map(str::to_owned) {
                        Some(file) => {
                            let read = cx
                                .background_executor()
                                .spawn(async move { std::fs::read_to_string(file) })
                                .await;
                            read.ok()
                                .map(|html| (version, RichBlock::new(RichKind::Html, html)))
                        }
                        None => None,
                    }
                }
                Err(_) => None,
            };
            let _ = this.update(cx, |s, cx| {
                s.asking.remove(&id);
                if let Some(found) = found {
                    s.blocks.insert(id, found);
                    cx.notify();
                }
            });
        })
        .detach();
        None
    }
}

// ---------------------------------------------------------------- the plugin

/// The kit's block plugin: a closed ```print fence naming a Print, while
/// Prints are on, is drawn as its card.
pub(crate) struct PrintCards;

#[derive(Clone)]
struct CardRef(String);

/// The Print id a code node names, once its closing fence has come.
fn closed_print(node: &markdown::mdast::Node, source: &str) -> Option<String> {
    let markdown::mdast::Node::Code(code) = node else {
        return None;
    };
    let language = match (&code.lang, &code.meta) {
        (Some(lang), Some(meta)) => format!("{lang} {meta}"),
        (Some(lang), None) => lang.clone(),
        _ => return None,
    };
    let id = core::fence_id(Some(&language), &code.value)?;
    let at = code.position.as_ref()?;
    threadbear_core::draft_block::fence_closed(source, at.start.offset, at.end.offset).then_some(id)
}

/// Whether Prints are on, which decides if a fence is a card at all.
fn on(cx: &App) -> bool {
    store(cx)
        .and_then(|s| s.read(cx).app.as_ref()?.upgrade())
        .is_some_and(|app| app.read(cx).prints_on())
}

impl MarkdownPlugin for PrintCards {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        NODE
    }

    fn parse(
        &self,
        node: &markdown::mdast::Node,
        cx: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let id = closed_print(node, cx.source())?;
        let source = cx.node_source(node).unwrap_or_default().to_owned();
        Some(
            MarkdownNode::new(NODE, CardRef(id.clone()))
                .text(format!("```print {id}\n```"))
                .markdown(source)
                .accessibility_label(format!("Print {id}")),
        )
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match node.data::<CardRef>() {
            Some(CardRef(id)) if on(cx) => card(id, window, cx),
            // Prints off: the fence as it was written.
            _ => div()
                .w_full()
                .my(px(4.))
                .p(px(PAD))
                .rounded(px(8.))
                .bg(style::code_background(cx))
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(size::CODE))
                .text_color(style::secondary_text(cx))
                .child(node.as_text().to_owned())
                .into_any_element(),
        }
    }
}

/// What a card shows of its Print, read from the app.
struct Shown {
    page: Option<Page>,
    refreshing: bool,
    busy: bool,
}

fn shown(id: &str, cx: &App) -> (Option<Entity<Threadbear>>, Shown) {
    let app = store(cx).and_then(|s| s.read(cx).app.as_ref()?.upgrade());
    let shown = match &app {
        Some(app) => {
            let a = app.read(cx);
            let page = a
                .prints
                .list
                .iter()
                .find(|p| p.id == id && p.deleted_at.is_none())
                .cloned();
            Shown {
                refreshing: page.as_ref().is_some_and(|p| a.print_refreshing(p)),
                busy: a.print_busy(id),
                page,
            }
        }
        None => Shown {
            page: None,
            refreshing: false,
            busy: false,
        },
    };
    (app, shown)
}

/// The card (PageCardCell).
fn card(id: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let (
        app,
        Shown {
            page,
            refreshing,
            busy,
        },
    ) = shown(id, cx);
    let now = now_ms();
    let (fg, secondary, faint) = (
        style::label_text(cx),
        style::secondary_text(cx),
        style::faint_text(cx),
    );
    let accent = style::accent(cx);
    let new_version = page
        .as_ref()
        .is_some_and(|p| p.is_new_version(refreshing, now));
    let preview = page
        .as_ref()
        .map(|page| preview(page, refreshing, app.clone(), window, cx));
    let glyph = div()
        .flex_none()
        .size(px(30.))
        .rounded(px(7.))
        .bg(fg.opacity(0.08))
        .flex()
        .items_center()
        .justify_center()
        .child(
            icons::symbol(page.as_ref().map_or("questionmark.circle", |p| p.symbol()))
                .with_size(px(13.))
                .text_color(secondary),
        );
    let meta = page.as_ref().map_or_else(
        || core::NOT_HERE.to_owned(),
        |p| p.card_meta(refreshing, now),
    );
    let title = h_flex()
        .min_w_0()
        .gap(px(8.))
        .items_center()
        .child(
            div()
                .flex_shrink(1.)
                .min_w_0()
                .truncate()
                .text_size(px(size::LABEL))
                .line_height(px(17.))
                .font_semibold()
                .text_color(fg)
                .child(
                    page.as_ref()
                        .map_or_else(|| id.to_owned(), |p| p.title.clone()),
                ),
        )
        .when(new_version, |s| {
            s.child(
                div()
                    .flex_none()
                    .px(px(5.))
                    .rounded(px(4.))
                    .border(gpui_kit::px(0.75))
                    .border_color(accent.opacity(0.5))
                    .text_size(px(size::DETAIL - 1.))
                    .line_height(px(15.))
                    .font_medium()
                    .text_color(accent)
                    .child("New version"),
            )
        });
    let words = v_flex().flex_1().min_w_0().child(title).child(
        div()
            .truncate()
            .text_size(px(size::DETAIL))
            .line_height(px(15.))
            .text_color(secondary)
            .child(meta),
    );
    let mut footer = h_flex()
        .h(px(FOOTER))
        .flex_none()
        .px(px(PAD))
        .gap(px(10.))
        .items_center()
        .child(glyph)
        .child(words);
    if let (Some(app), Some(page)) = (app.clone(), page.as_ref()) {
        let pid = page.id.clone();
        footer = footer
            .child(button(
                format!("print-open-{pid}"),
                "arrow.up.forward.square",
                Some("Open"),
                OPEN_TIP,
                true,
                busy,
                cx,
                {
                    let (app, pid) = (app.clone(), pid.clone());
                    move |_, window, cx| app.update(cx, |a, cx| a.open_print_here(&pid, window, cx))
                },
            ))
            .child(button(
                format!("print-refresh-{pid}"),
                "arrow.clockwise",
                Some("Refresh"),
                REFRESH_TIP,
                false,
                busy || refreshing,
                cx,
                {
                    let (app, pid) = (app.clone(), pid.clone());
                    move |_, window, cx| {
                        app.update(cx, |a, cx| {
                            a.run_print_action(PrintAction::Refresh(pid.clone()), window, cx)
                        })
                    }
                },
            ))
            .child(button(
                format!("print-more-{pid}"),
                "ellipsis",
                None,
                MORE_TIP,
                false,
                false,
                cx,
                {
                    let (app, pid) = (app.clone(), pid.clone());
                    move |ev, window, cx| {
                        let at = ev.position();
                        app.update(cx, |a, cx| {
                            a.open_print_menu(
                                &pid,
                                true,
                                point(at.x, at.y + gpui_kit::px(14.)),
                                window,
                                cx,
                            )
                        })
                    }
                },
            ));
    }
    v_flex()
        .id(SharedString::from(format!("print-card-{id}")))
        .test_support()
        .w_full()
        .my(px(4.))
        .rounded(px(RADIUS))
        .bg(style::code_background(cx))
        .border(gpui_kit::px(if new_version { 1. } else { 0.5 }))
        .border_color(if new_version {
            accent.opacity(0.55)
        } else {
            style::card_border(cx)
        })
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when_some(preview, |s, p| {
            s.child(p)
                .child(div().h(gpui_kit::px(0.5)).w_full().bg(style::hairline(cx)))
        })
        .child(footer)
        .text_color(faint)
        .into_any_element()
}

/// The top of the newest page as a picture; a click opens it beside the
/// thread.
fn preview(
    page: &Page,
    refreshing: bool,
    app: Option<Entity<Threadbear>>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let block = store(cx).and_then(|s| s.update(cx, |s, cx| s.block(page, cx)));
    let thumb = block
        .as_ref()
        .map_or(Thumb::Waiting, |b| crate::rich_boxes::thumb(b, window, cx));
    let secondary = style::secondary_text(cx);
    let note = |text: String| {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .px(px(PAD))
            .text_size(px(size::DETAIL))
            .text_color(secondary)
            .child(text)
    };
    let body = match thumb {
        Thumb::Picture(picture, width, height) => {
            let ratio = if height > 0. { width / height } else { 1. };
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .opacity(if refreshing { 0.35 } else { 1. })
                .child(
                    div().w_full().aspect_ratio(ratio).child(
                        img(ImageSource::Image(picture))
                            .size_full()
                            .object_fit(ObjectFit::Fill),
                    ),
                )
                .into_any_element()
        }
        Thumb::Waiting => note("Drawing a preview…".into()).into_any_element(),
        Thumb::NoRuntime => {
            note("The preview needs the web runtime. Open shows the print itself.".into())
                .into_any_element()
        }
        Thumb::Failed(why) => note(format!("Couldn't draw a preview: {why}")).into_any_element(),
    };
    let pid = page.id.clone();
    div()
        .id(SharedString::from(format!("print-preview-{pid}")))
        .role(Role::Button)
        .relative()
        .w_full()
        .h(px(PREVIEW))
        .flex_none()
        .overflow_hidden()
        .cursor_pointer()
        .tip("Open this print")
        .on_click(move |_, window, cx| {
            if let Some(app) = &app {
                app.update(cx, |a, cx| a.open_print_here(&pid, window, cx));
            }
        })
        .child(body)
        .when(refreshing, |s| {
            s.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(size::DETAIL))
                    .font_medium()
                    .text_color(style::label_text(cx))
                    .child("Refreshing in its thread…"),
            )
        })
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn button(
    id: String,
    symbol: &'static str,
    label: Option<&'static str>,
    tip: &'static str,
    strong: bool,
    off: bool,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let fg = style::label_text(cx);
    let color = if off { style::faint_text(cx) } else { fg };
    // Tipped as the app's buttons are (`tipped`): a button to VoiceOver,
    // and the label scripts find it by.
    let el = div()
        .id(SharedString::from(id))
        .flex_none()
        .h(px(26.))
        .px(px(if label.is_some() { 8. } else { 6. }))
        .gap(px(4.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .bg(fg.opacity(if strong { 0.12 } else { 0.06 }))
        .border(gpui_kit::px(0.5))
        .border_color(fg.opacity(0.14))
        .text_color(color)
        .text_size(px(size::DETAIL))
        .font_medium()
        .when(!off, |s| {
            s.cursor_pointer()
                .hover(move |s| s.bg(fg.opacity(if strong { 0.18 } else { 0.1 })))
                .on_click(on_click)
        })
        .child(icons::symbol(symbol).with_size(px(12.)))
        .when_some(label, |s, l| s.child(l));
    crate::highlights::tipped(el, tip).into_any_element()
}

// ---------------------------------------------------------------- the dump

/// A reply's Print blocks as drawn, for the specs (`prints` on a row): a
/// card with its title, words and buttons, or `shows: "code"` for one still
/// streaming in or with Prints off.
#[cfg(feature = "automation")]
pub(crate) fn dump(text: &str, cx: &App) -> Option<Value> {
    if !text.contains("```") && !text.contains("~~~") {
        return None;
    }
    let Ok(tree) = markdown::to_mdast(text, &markdown::ParseOptions::gfm()) else {
        return None;
    };
    let mut found = Vec::new();
    collect(&tree, text, &mut found);
    if found.is_empty() {
        return None;
    }
    let (now, prints_on) = (now_ms(), on(cx));
    Some(Value::Array(
        found
            .into_iter()
            .map(|(id, closed)| {
                if !(closed && prints_on) {
                    return json!({ "id": id, "shows": "code" });
                }
                let (_, s) = shown(&id, cx);
                match s.page {
                    Some(p) => json!({ "id": id, "shows": "card", "title": p.title, "meta": p.card_meta(s.refreshing, now), "newVersion": p.is_new_version(s.refreshing, now), "version": p.version, "buttons": ["Open", "Refresh"], "menu": true }),
                    None => json!({ "id": id, "shows": "card", "meta": core::NOT_HERE, "buttons": [] }),
                }
            })
            .collect(),
    ))
}

/// Each ```print fence's id, and whether it's closed yet.
#[cfg(feature = "automation")]
fn collect(node: &markdown::mdast::Node, text: &str, found: &mut Vec<(String, bool)>) {
    if let markdown::mdast::Node::Code(code) = node {
        let language = match (&code.lang, &code.meta) {
            (Some(lang), Some(meta)) => format!("{lang} {meta}"),
            (Some(lang), None) => lang.clone(),
            _ => String::new(),
        };
        if let Some(id) = core::fence_id(Some(&language), &code.value) {
            found.push((id, closed_print(node, text).is_some()));
        }
        return;
    }
    for child in node.children().into_iter().flatten() {
        collect(child, text, found);
    }
}
