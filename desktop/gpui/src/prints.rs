// Prints in a reply (prints.card, prints.pane): a ```print <id> block drawn
// as the Print's card, and Open putting the Print in the thread's side pane,
// which moves on to each new version as it's saved. A port of the Swift
// app's PageCards and PageCardCell (Transcript/PageCards.swift, "Preview on
// top", Wayne picked it on 10 Nov 2026) and openPageHere / followPageVersions.
//
// The store is the runner's Prints as `pages.list` and `pages.updated` give
// them, and the newest file of each that a card has asked for (`pages.get`).
// The card's picture is the top of that file, drawn by the web runtime as an
// ```html block is (rich_boxes.rs `wanted`): the Print's own index.html,
// under the same lockdown. Without the runtime it says so. The pane shows the
// file through the local-page path every local page takes (`file.htmlpreview`).
//
// Not here yet (2.1c, prints.menu): the card's ⋯ menu.
use crate::style::{self, px, size};
use gpui_kit::base::text::{MarkdownNode, MarkdownParseContext, MarkdownPlugin};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use threadbear_client::runner::Client;
use threadbear_core::prints::{self as core, Page};
use threadbear_core::rich_block::{RichBlock, RichKind};

/// What the kit calls the card's nodes.
const NODE: &str = "print-card";

/// The card's measures at 100% (PageCardCell's).
const PREVIEW_HEIGHT: f32 = 170.;
const FOOTER_HEIGHT: f32 = 56.;
const PAD: f32 = 12.;
const RADIUS: f32 = 10.;

/// The biggest index.html read for a picture: the runner's own limit.
const FILE_MAX: u64 = 1024 * 1024;
/// How long Refresh counts as under way without a new version (`refreshFor`).
const REFRESH_FOR: Duration = Duration::from_secs(20 * 60);

/// The buttons' tips, which scripts find them by (PageCardCell.Action.tip,
/// with Open saying where it goes here).
pub(crate) const OPEN_TIP: &str = "Open this print beside the thread";
pub(crate) const PREVIEW_TIP: &str = "Open this print";
pub(crate) const REFRESH_TIP: &str = "Ask the thread that made it to make it again the same way";

// ---------------------------------------------------------------- the store

pub(crate) struct PrintStore {
    /// The experiment's on (hello's `experiments`): with it off a block stays code.
    pub(crate) enabled: bool,
    client: Option<Arc<Client>>,
    app: Option<WeakEntity<crate::app::Threadbear>>,
    pages: Vec<Page>,
    /// The newest file of each Print a card asked for, at the version asked.
    files: HashMap<String, (i64, String)>,
    /// Each file's page, for its picture, by path.
    sources: HashMap<String, Option<RichBlock>>,
    asking: HashSet<String>,
    /// Open asked for and not answered yet: the button shows it and takes no clicks.
    opening: HashSet<String>,
    /// Refresh sent at this version (`refreshing`), until a newer one lands.
    refreshing: HashMap<String, (i64, Instant)>,
}

struct Shared(Entity<PrintStore>);
impl Global for Shared {}

pub(crate) fn init(app: WeakEntity<crate::app::Threadbear>, cx: &mut App) {
    let store = cx.new(|_| PrintStore {
        enabled: false,
        client: None,
        app: Some(app),
        pages: Vec::new(),
        files: HashMap::new(),
        sources: HashMap::new(),
        asking: HashSet::new(),
        opening: HashSet::new(),
        refreshing: HashMap::new(),
    });
    cx.set_global(Shared(store));
}

pub(crate) fn store(cx: &App) -> Option<Entity<PrintStore>> {
    cx.try_global::<Shared>().map(|s| s.0.clone())
}

/// Connected: whether Prints is on, and the list (`loadPages`).
pub(crate) fn connected(client: Arc<Client>, experiments: &[String], cx: &mut App) {
    let Some(store) = store(cx) else { return };
    let enabled = experiments.iter().any(|e| e == core::EXPERIMENT);
    store.update(cx, |s, cx| {
        s.enabled = enabled;
        s.client = Some(client.clone());
        if !enabled {
            s.pages.clear();
        }
        cx.notify();
    });
    if !enabled {
        return;
    }
    cx.spawn(async move |cx| {
        // An older runner, or the experiment just turned off: none.
        let pages = crate::call(client, "pages.list", json!({})).await.map(|v| Page::list(&v)).unwrap_or_default();
        let _ = store.update(cx, |s, cx| {
            s.pages = pages;
            cx.notify();
        });
    })
    .detach();
}

/// `pages.updated`: every card shows the newest, and a pane showing one moves on.
pub(crate) fn updated(params: &Value, side: &Entity<crate::highlights::SidePane>, cx: &mut App) {
    let Some(store) = store(cx) else { return };
    let after = Page::list(params);
    let before = store.update(cx, |s, cx| {
        let before = std::mem::replace(&mut s.pages, after.clone());
        // A newer version than Refresh was sent at: it's done.
        s.refreshing.retain(|id, (version, _)| !after.iter().any(|p| &p.id == id && p.version > *version));
        cx.notify();
        before
    });
    side.update(cx, |side, cx| side.follow_print_versions(&before, &after, cx));
}

impl PrintStore {
    fn page(&self, id: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id && !p.deleted)
    }

    fn is_refreshing(&self, page: &Page) -> bool {
        self.refreshing.get(&page.id).is_some_and(|(v, at)| page.version <= *v && at.elapsed() < REFRESH_FOR)
    }

    /// The page the card's picture draws, once its newest file's path and
    /// text are in; asked for from the runner the first time (`thumb`).
    fn picture_block(&mut self, page: &Page, cx: &mut Context<Self>) -> Option<RichBlock> {
        if let Some((version, file)) = self.files.get(&page.id).filter(|(v, _)| *v == page.version) {
            let _ = version;
            if let Some(block) = self.sources.get(file) {
                return block.clone();
            }
            let file = file.clone();
            self.sources.insert(file.clone(), None);
            cx.spawn(async move |this, cx| {
                let read = file.clone();
                let text = cx.background_executor().spawn(async move { read_page(&read) }).await;
                let _ = this.update(cx, |s, cx| {
                    s.sources.insert(file, text.map(|t| RichBlock::new(RichKind::Html, t)));
                    cx.notify();
                });
            })
            .detach();
            return None;
        }
        if !self.asking.insert(page.id.clone()) {
            return None;
        }
        let Some(client) = self.client.clone() else { return None };
        let id = page.id.clone();
        cx.spawn(async move |this, cx| {
            let got = crate::call(client, "pages.get", json!({ "pageId": id })).await;
            let _ = this.update(cx, |s, cx| {
                s.asking.remove(&id);
                if let Ok(v) = got {
                    if let (Some(page), Some(file)) = (Page::decode(&v["page"]), v["file"].as_str()) {
                        s.files.insert(id, (page.version, file.to_owned()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        None
    }

    /// Open: the newest version's file in the thread's side pane
    /// (`openPageHere`), the button busy until the runner answers.
    fn open(&mut self, id: String, thread_id: String, cx: &mut Context<Self>) {
        let (Some(client), Some(app)) = (self.client.clone(), self.app.clone()) else { return };
        if !self.opening.insert(id.clone()) {
            return;
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let got = crate::call(client, "pages.get", json!({ "pageId": id })).await;
            let _ = this.update(cx, |s, cx| {
                s.opening.remove(&id);
                cx.notify();
            });
            let file = match got {
                Ok(v) => v["file"].as_str().map(|f| (f.to_owned(), v["page"]["title"].as_str().map(str::to_owned))),
                Err(e) => {
                    let _ = app.update(cx, |app, cx| app.show_error(format!("Couldn't open the print: {}", e.message), cx));
                    return;
                }
            };
            let Some((file, title)) = file else { return };
            // The side pane reads the app as it opens a file, so it's
            // updated on its own, not inside the app's update.
            let Ok((Some(thread), side)) = app.read_with(cx, |app, _| (app.threads.get(&thread_id).cloned(), app.side.clone())) else { return };
            let section = side.update(cx, |side, cx| {
                side.preview_path(&file, title, &thread, cx);
                side.section_for(&thread)
            });
            let _ = app.update(cx, |app, cx| {
                if !app.dock.read(cx).controller.is_showing(&section) {
                    app.show_pane(&section, cx);
                }
            });
        })
        .detach();
    }

    /// Refresh: the thread that made it is asked to make it again
    /// (`pages.refresh`); the card says so until a new version lands.
    fn refresh(&mut self, page: &Page, cx: &mut Context<Self>) {
        let (Some(client), Some(app)) = (self.client.clone(), self.app.clone()) else { return };
        self.refreshing.insert(page.id.clone(), (page.version, Instant::now()));
        cx.notify();
        let id = page.id.clone();
        cx.spawn(async move |this, cx| {
            if let Err(e) = crate::call(client, "pages.refresh", json!({ "pageId": id })).await {
                let _ = this.update(cx, |s, cx| {
                    s.refreshing.remove(&id);
                    cx.notify();
                });
                let _ = app.update(cx, |app, cx| app.show_error(format!("Couldn't refresh the print: {}", e.message), cx));
            }
        })
        .detach();
    }
}

/// A Print's index.html, if it's there and no bigger than the runner allows.
fn read_page(file: &str) -> Option<String> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() > FILE_MAX {
        return None;
    }
    std::fs::read_to_string(file).ok()
}

// ---------------------------------------------------------------- the plugin

/// The kit's block plugin: a closed ```print fence naming a Print is its
/// card; anything else, or one still streaming in, stays the kit's own.
pub(crate) struct PrintCards {
    /// The thread whose reply it is, whose side pane Open uses.
    pub(crate) thread: String,
}

#[derive(Clone)]
struct PrintRef(String);

impl MarkdownPlugin for PrintCards {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        NODE
    }

    fn parse(&self, node: &markdown::mdast::Node, cx: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let id = closed_fence(node, cx.source())?;
        let source = cx.node_source(node).unwrap_or_default().to_owned();
        Some(MarkdownNode::new(NODE, PrintRef(id.clone())).text(source.clone()).markdown(source).accessibility_label(format!("Print {id}")))
    }

    fn render(&self, node: &MarkdownNode, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match node.data::<PrintRef>() {
            Some(PrintRef(id)) => card(id, &self.thread, node.as_text(), window, cx),
            None => div().child(node.as_text().to_owned()).into_any_element(),
        }
    }
}

/// The Print a code node names, once its closing fence has come.
fn closed_fence(node: &markdown::mdast::Node, source: &str) -> Option<String> {
    let markdown::mdast::Node::Code(code) = node else { return None };
    // mdast splits the info string: `print` is its lang, the id its meta.
    let info = [code.lang.as_deref(), code.meta.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let id = core::fence_id(Some(&info), &code.value)?;
    let at = code.position.as_ref()?;
    threadbear_core::draft_block::fence_closed(source, at.start.offset, at.end.offset).then_some(id)
}

/// The Prints a reply's closed fences name, in order.
fn fences_in(text: &str) -> Vec<String> {
    if !core::may_hold_fence(text) {
        return Vec::new();
    }
    let Ok(tree) = markdown::to_mdast(text, &markdown::ParseOptions::gfm()) else { return Vec::new() };
    let mut found = Vec::new();
    collect(&tree, text, &mut found);
    found
}

fn collect(node: &markdown::mdast::Node, text: &str, found: &mut Vec<String>) {
    if let Some(id) = closed_fence(node, text) {
        found.push(id);
        return;
    }
    for child in node.children().into_iter().flatten() {
        collect(child, text, found);
    }
}

// ---------------------------------------------------------------- the card

/// Where the card's picture has got to, for the card and the dump.
enum Picture {
    Drawn { ratio: f32, image: Arc<Image> },
    Waiting,
    NoRuntime,
    Failed(String),
}

fn picture(store: &Entity<PrintStore>, page: &Page, window: &mut Window, cx: &mut App) -> Picture {
    let Some(block) = store.update(cx, |s, cx| s.picture_block(page, cx)) else { return Picture::Waiting };
    let Some(rich) = crate::rich_boxes::store(cx) else { return Picture::NoRuntime };
    let (state, image) = crate::rich_boxes::wanted(&rich, &block, true, window, cx);
    match (state, image) {
        (Some(crate::rich_boxes::Drawing::Drawn { width, height, .. }), Some(image)) => Picture::Drawn { ratio: if height > 0. { width / height } else { 1. }, image },
        (Some(crate::rich_boxes::Drawing::Failed(why)), _) => Picture::Failed(why),
        (None, _) if !crate::web_runtime::runtime(cx).is_some_and(|r| r.read(cx).state == threadbear_core::web_runtime::State::Ready) => Picture::NoRuntime,
        _ => Picture::Waiting,
    }
}

/// The card (PageCardCell): the top of the Print as a picture, and under it
/// the kind's glyph, the title and what it is, and Open and Refresh.
fn card(id: &str, thread: &str, fenced: &str, window: &mut Window, cx: &mut App) -> AnyElement {
    let Some(store) = store(cx).filter(|s| s.read(cx).enabled) else { return as_code(fenced, cx) };
    let (page, refreshing, opening) = {
        let s = store.read(cx);
        let page = s.page(id).cloned();
        let refreshing = page.as_ref().is_some_and(|p| s.is_refreshing(p));
        (page, refreshing, s.opening.contains(id))
    };
    let meta = core::card_meta_now(page.as_ref(), refreshing);
    let new_version = page.as_ref().is_some_and(|p| core::is_new_version(p, refreshing, core::now_ms()));
    let (fill, edge, hairline, secondary, label) = (box_fill(cx), style::card_border(cx), style::hairline(cx), style::secondary_label(cx), style::label_text(cx));
    let accent = style::accent(cx);
    let open = {
        let (store, id, thread) = (store.clone(), id.to_owned(), thread.to_owned());
        move |cx: &mut App| store.update(cx, |s, cx| s.open(id.clone(), thread.clone(), cx))
    };
    let preview = page.as_ref().map(|page| {
        let body = match picture(&store, page, window, cx) {
            Picture::Drawn { ratio, image } => div()
                .w_full()
                .aspect_ratio(ratio)
                .when(refreshing, |d| d.opacity(0.35))
                .child(img(ImageSource::Image(image)).size_full().object_fit(ObjectFit::Fill))
                .into_any_element(),
            other => {
                let words = match other {
                    Picture::NoRuntime => "No preview until the web runtime is installed".to_owned(),
                    Picture::Failed(why) => format!("Couldn't draw a preview: {why}"),
                    _ => "Drawing a preview…".to_owned(),
                };
                div().size_full().flex().items_center().justify_center().text_size(px(size::DETAIL)).text_color(secondary).child(words).into_any_element()
            }
        };
        let open = open.clone();
        let el = div()
            .id(SharedString::from(format!("print-preview-{}", page.id)))
            .relative()
            .w_full()
            .h(px(PREVIEW_HEIGHT))
            .overflow_hidden()
            .cursor_pointer()
            .role(Role::Button)
            .aria_label(PREVIEW_TIP)
            .child(body)
            .when(refreshing, |d| {
                d.child(div().absolute().inset_0().flex().items_center().justify_center().text_size(px(size::DETAIL)).font_weight(FontWeight::MEDIUM).text_color(label).child("Refreshing in its thread…"))
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, _, cx| open(cx));
        crate::highlights::tipped(el, PREVIEW_TIP).into_any_element()
    });
    let glyph = div()
        .flex_none()
        .size(px(30.))
        .rounded(px(7.))
        .bg(style::primary_opacity(cx, 0.08))
        .flex()
        .items_center()
        .justify_center()
        .text_color(secondary)
        .child(crate::icons::symbol(symbol(page.as_ref())).size(px(size::DETAIL)));
    let title = h_flex()
        .gap(px(8.))
        .min_w_0()
        .child(div().min_w_0().truncate().text_size(px(size::LABEL)).font_weight(FontWeight::SEMIBOLD).text_color(label).child(page.as_ref().map_or_else(|| id.to_owned(), |p| p.title.clone())))
        .when(new_version, |d| {
            d.child(div().flex_none().px(px(5.)).rounded(px(4.)).border(gpui_kit::px(0.75)).border_color(accent.opacity(0.5)).text_size(px(size::DETAIL - 1.)).font_weight(FontWeight::MEDIUM).text_color(accent).child("New version"))
        });
    let words = v_flex().flex_1().min_w_0().child(title).child(div().min_w_0().truncate().text_size(px(size::DETAIL)).text_color(secondary).child(meta));
    let buttons = page.as_ref().map(|page| {
        let refresh = {
            let (store, page) = (store.clone(), page.clone());
            move |cx: &mut App| store.update(cx, |s, cx| s.refresh(&page, cx))
        };
        h_flex()
            .flex_none()
            .gap(px(6.))
            .child(button(format!("print-open-{}", page.id), if opening { "hourglass" } else { "arrow.up.forward.square" }, if opening { "Opening…" } else { "Open" }, OPEN_TIP, true, !opening, cx, open.clone()))
            .child(button(format!("print-refresh-{}", page.id), "arrow.clockwise", "Refresh", REFRESH_TIP, false, !refreshing, cx, refresh))
    });
    let footer = h_flex().h(px(FOOTER_HEIGHT)).px(px(PAD)).gap(px(10.)).items_center().child(glyph).child(words).children(buttons);
    v_flex()
        .id(SharedString::from(format!("print-card-{id}")))
        .w_full()
        .my(px(4.))
        .rounded(px(RADIUS))
        .bg(fill)
        // A new version's ring, else the hairline edge every card has.
        .when(new_version, |d| d.border_1().border_color(accent.opacity(0.55)))
        .when(!new_version, |d| d.border(gpui_kit::px(0.5)).border_color(edge))
        .overflow_hidden()
        // Not the start of a selection in the reply around it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when_some(preview, |d, preview| d.child(preview).child(div().h(gpui_kit::px(0.5)).w_full().bg(hairline)))
        .child(footer)
        .into_any_element()
}

/// A footer button: a glyph and its word, Open a shade stronger. One that's
/// busy is dimmed and takes no clicks.
#[allow(clippy::too_many_arguments)]
fn button(id: String, symbol: &'static str, title: &'static str, tip: &'static str, strong: bool, enabled: bool, cx: &App, action: impl Fn(&mut App) + 'static) -> AnyElement {
    let fg = if enabled { style::label_text(cx) } else { style::tertiary_label(cx) };
    let hover = style::primary_opacity(cx, if strong { 0.18 } else { 0.1 });
    let el = div()
        .id(SharedString::from(id))
        .h(px(26.))
        .pl(px(6.))
        .pr(px(9.))
        .gap(px(4.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .bg(style::primary_opacity(cx, if strong { 0.12 } else { 0.06 }))
        .border(gpui_kit::px(0.5))
        .border_color(style::primary_opacity(cx, 0.14))
        .text_size(px(size::DETAIL))
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .when(enabled, |d| d.cursor_pointer().hover(move |s| s.bg(hover)))
        .child(crate::icons::symbol(symbol).size(px(size::DETAIL)))
        .child(title)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when(enabled, |d| d.on_click(move |_, _, cx| action(cx)));
    crate::highlights::tipped(el, tip).into_any_element()
}

/// The kind's glyph, from the ones the app has (the Swift app's SF Symbols
/// for chart, dashboard, diagram and board aren't among them).
fn symbol(page: Option<&Page>) -> &'static str {
    match page.map(|p| p.symbol()) {
        None => "questionmark.circle",
        Some("chart.bar" | "rectangle.3.group") => "square.grid.2x2",
        Some("doc.text.magnifyingglass") => "doc.text",
        Some("point.3.connected.trianglepath.dotted") => "arrow.triangle.branch",
        Some("shippingbox") => "shippingbox.fill",
        Some("rectangle.split.3x1") => "rectangle.split.2x1",
        Some(other @ ("doc.plaintext" | "checklist")) => other,
        Some(_) => "doc.richtext",
    }
}

/// The card's fill: the code box's, opaque (Theme.codeBackgroundOpaque).
fn box_fill(cx: &App) -> Hsla {
    if cx.theme().is_dark() {
        rgb(0x282828).into()
    } else {
        rgb(0xf7f7f7).into()
    }
}

/// With Prints off the block is what it says, as code.
fn as_code(fenced: &str, cx: &App) -> AnyElement {
    div()
        .w_full()
        .my(px(4.))
        .p(px(PAD))
        .rounded(px(8.))
        .bg(style::code_background(cx))
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(size::DETAIL))
        .whitespace_normal()
        .child(fenced.to_owned())
        .into_any_element()
}

// ---------------------------------------------------------------- the dump

/// A reply's Print cards as drawn: the title, the line under it, the picture's
/// state and the buttons, for the specs (`prints` on a row).
#[cfg(feature = "automation")]
pub(crate) fn dump(text: &str, cx: &App) -> Option<Value> {
    let ids = fences_in(text);
    if ids.is_empty() {
        return None;
    }
    let store = store(cx)?;
    let s = store.read(cx);
    if !s.enabled {
        return Some(json!(ids.iter().map(|id| json!({ "id": id, "shows": "code" })).collect::<Vec<_>>()));
    }
    let cards = ids
        .iter()
        .map(|id| {
            let page = s.page(id);
            let refreshing = page.is_some_and(|p| s.is_refreshing(p));
            // How far its picture has got: the file asked for, read, then drawn.
            let picture = match page.and_then(|p| s.files.get(&p.id).filter(|(v, _)| *v == p.version)).and_then(|(_, f)| s.sources.get(f)) {
                Some(Some(block)) => match crate::rich_boxes::store(cx).and_then(|r| r.read(cx).state(block, style::zoom_percent()).cloned()) {
                    Some(crate::rich_boxes::Drawing::Drawn { .. }) => "drawn".to_owned(),
                    Some(crate::rich_boxes::Drawing::Failed(why)) => format!("failed: {why}"),
                    Some(_) => "drawing".to_owned(),
                    None => "read".to_owned(),
                },
                Some(None) => "reading".to_owned(),
                None => "asking".to_owned(),
            };
            json!({
                "id": id,
                "shows": "card",
                "title": page.map(|p| p.title.clone()),
                "meta": core::card_meta_now(page, refreshing),
                "version": page.map(|p| p.version),
                "newVersion": page.is_some_and(|p| core::is_new_version(p, refreshing, core::now_ms())),
                "picture": picture,
                "opening": s.opening.contains(id),
                "buttons": if page.is_some() { json!(["Open", "Refresh"]) } else { json!([]) },
            })
        })
        .collect::<Vec<_>>();
    Some(json!(cards))
}
