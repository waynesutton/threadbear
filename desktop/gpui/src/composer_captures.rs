// Pending "Add from page" captures waiting in a thread's composer before Send
// (agent_browser.rs, threadbear_core::browserbear). Every capture is pinned to
// the thread that started the pick, deduplicated by `capture_id`, and never
// auto-sent: the user can inspect the cropped screenshot, sanitized URL, quote
// and collapsed metadata, remove the quote or screenshot, discard the capture,
// or press the existing Send button.
use crate::composer::{self, Composer};
use crate::spinner::Spinner;
use crate::style::{self, px, size};
use crate::tips::Tip as _;
use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{h_flex, v_flex, Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::Arc;
use threadbear_client::runner::Client;
use threadbear_core::browserbear::{CaptureStore, PickCapture, PickImageFormat};
use threadbear_core::reducer::ImageRef;

pub(crate) const DISCARD_TIP: &str = "Discard this page capture";
pub(crate) const DETAILS_SHOW_TIP: &str = "Show element details";
pub(crate) const DETAILS_HIDE_TIP: &str = "Hide element details";
pub(crate) const REMOVE_QUOTE_TIP: &str = "Remove quoted text from this capture";
pub(crate) const REMOVE_IMAGE_TIP: &str = "Remove screenshot from this capture";
pub(crate) const SECRET_WARNING: &str =
    "Screenshots can still show secrets visible on the page. Review before sending.";

#[derive(Clone)]
pub(crate) struct PendingCaptureMeta {
    pub(crate) thumbnail: Option<Arc<Image>>,
    pub(crate) importing: bool,
    pub(crate) image: Option<ImageRef>,
    pub(crate) import_error: Option<String>,
    pub(crate) expanded: bool,
}

#[derive(Default)]
struct PageCaptures(CaptureStore<PendingCaptureMeta>);
impl Global for PageCaptures {}

fn store_mut(cx: &mut App) -> &mut CaptureStore<PendingCaptureMeta> {
    if cx.try_global::<PageCaptures>().is_none() {
        cx.set_global(PageCaptures::default());
    }
    &mut cx.global_mut::<PageCaptures>().0
}

fn store(cx: &App) -> Option<&CaptureStore<PendingCaptureMeta>> {
    cx.try_global::<PageCaptures>().map(|g| &g.0)
}

/// Inserts a validated capture into its pinned thread's composer and starts
/// importing its screenshot through the runner's `media.import` (reusing the
/// runner's attachment limits). Returns `false` if `capture_id` was already
/// accepted.
pub(crate) fn insert(capture: PickCapture, client: Arc<Client>, cx: &mut App) -> bool {
    let thread = capture.thread.clone();
    let capture_id = capture.capture_id.clone();
    let staged = capture.screenshot.as_ref().map(|shot| {
        let format = match shot.format {
            PickImageFormat::Png => ImageFormat::Png,
            PickImageFormat::Jpeg => ImageFormat::Jpeg,
        };
        let thumb = Arc::new(Image::from_bytes(format, shot.bytes.clone()));
        let name = format!("page-capture.{}", shot.format.extension());
        let path = composer::stage_private(&composer::paste_base(), &name, &shot.bytes);
        (thumb, path)
    });
    let (thumbnail, importing, import_error, staged_path) = match staged {
        Some((thumb, Some(path))) => (Some(thumb), true, None, Some(path)),
        Some((thumb, None)) => (
            Some(thumb),
            false,
            Some("Couldn't stage the screenshot for import.".to_owned()),
            None,
        ),
        None => (None, false, None, None),
    };
    let meta = PendingCaptureMeta {
        thumbnail,
        importing,
        image: None,
        import_error,
        expanded: false,
    };
    if !store_mut(cx).insert(capture, meta) {
        if let Some(path) = staged_path {
            composer::unstage(&path);
        }
        return false;
    }
    cx.refresh_windows();
    if let Some(path) = staged_path {
        cx.spawn(async move |cx| {
            let result = crate::call(
                client,
                "media.import",
                json!({ "path": path.to_string_lossy() }),
            )
            .await;
            composer::unstage(&path);
            cx.update(|cx| {
                if let Some(list) = store_mut(cx).for_thread_mut(&thread) {
                    if let Some((_, meta)) =
                        list.iter_mut().find(|(c, _)| c.capture_id == capture_id)
                    {
                        meta.importing = false;
                        match result {
                            Ok(v) => match ImageRef::decode(&v["image"]) {
                                Some(img) => {
                                    meta.image = Some(img);
                                    meta.import_error = None;
                                }
                                None => {
                                    meta.import_error = Some(
                                        "The runner returned an unreadable image reference."
                                            .to_owned(),
                                    );
                                }
                            },
                            Err(e) => {
                                meta.import_error =
                                    Some(format!("Couldn't attach the screenshot: {}", e.message));
                            }
                        }
                    }
                }
                cx.refresh_windows();
            });
        })
        .detach();
    }
    true
}

pub(crate) fn has_pending(thread: &str, cx: &App) -> bool {
    store(cx).is_some_and(|s| !s.for_thread(thread).is_empty())
}

pub(crate) fn is_importing(thread: &str, cx: &App) -> bool {
    store(cx).is_some_and(|s| s.for_thread(thread).iter().any(|(_, m)| m.importing))
}

pub(crate) fn discard(thread: &str, capture_id: &str, cx: &mut App) {
    if store_mut(cx).remove(thread, capture_id).is_some() {
        cx.refresh_windows();
    }
}

pub(crate) fn remove_quote(thread: &str, capture_id: &str, cx: &mut App) {
    let drop_whole = {
        let Some(list) = store_mut(cx).for_thread_mut(thread) else {
            return;
        };
        let Some((cap, meta)) = list.iter_mut().find(|(c, _)| c.capture_id == capture_id) else {
            return;
        };
        cap.quote = None;
        cap.screenshot.is_none() && meta.image.is_none()
    };
    if drop_whole {
        store_mut(cx).remove(thread, capture_id);
    }
    cx.refresh_windows();
}

pub(crate) fn remove_screenshot(thread: &str, capture_id: &str, cx: &mut App) {
    let drop_whole = {
        let Some(list) = store_mut(cx).for_thread_mut(thread) else {
            return;
        };
        let Some((cap, meta)) = list.iter_mut().find(|(c, _)| c.capture_id == capture_id) else {
            return;
        };
        cap.screenshot = None;
        meta.thumbnail = None;
        meta.image = None;
        meta.importing = false;
        meta.import_error = None;
        cap.warnings.retain(|w| w != "may_contain_visible_secrets");
        cap.quote.is_none()
    };
    if drop_whole {
        store_mut(cx).remove(thread, capture_id);
    }
    cx.refresh_windows();
}

pub(crate) fn toggle_details(thread: &str, capture_id: &str, cx: &mut App) {
    if let Some(list) = store_mut(cx).for_thread_mut(thread) {
        if let Some((_, meta)) = list.iter_mut().find(|(c, _)| c.capture_id == capture_id) {
            meta.expanded = !meta.expanded;
            cx.refresh_windows();
        }
    }
}

/// Takes all pending captures for `thread` when the user presses Send,
/// returning their formatted Markdown context, imported `ImageRef`s, and the
/// raw entries in case the send is refused and needs to be restored.
pub(crate) fn take_for_send(
    thread: &str,
    cx: &mut App,
) -> (
    String,
    Vec<ImageRef>,
    Vec<(PickCapture, PendingCaptureMeta)>,
) {
    let taken = store_mut(cx).take_thread(thread);
    if taken.is_empty() {
        return (String::new(), vec![], vec![]);
    }
    let blocks: Vec<String> = taken.iter().map(|(c, _)| c.message_block()).collect();
    let images: Vec<ImageRef> = taken.iter().filter_map(|(_, m)| m.image.clone()).collect();
    cx.refresh_windows();
    (blocks.join("\n\n"), images, taken)
}

pub(crate) fn restore_after_failed_send(
    thread: String,
    items: Vec<(PickCapture, PendingCaptureMeta)>,
    cx: &mut App,
) {
    if items.is_empty() {
        return;
    }
    store_mut(cx).restore_thread(thread, items);
    cx.refresh_windows();
}

impl Composer {
    pub(crate) fn has_pending_captures(&self, cx: &App) -> bool {
        match (&self.thread_id, &self.editing) {
            (Some(thread), None) => has_pending(thread, cx),
            _ => false,
        }
    }

    pub(crate) fn importing_captures(&self, cx: &App) -> bool {
        match (&self.thread_id, &self.editing) {
            (Some(thread), None) => is_importing(thread, cx),
            _ => false,
        }
    }

    pub(crate) fn render_capture_previews(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (Some(thread), None) = (&self.thread_id, &self.editing) else {
            return None;
        };
        let items = store(cx)?.for_thread(thread);
        if items.is_empty() {
            return None;
        }
        let mut list = v_flex().id("page-captures").w_full().gap(px(6.));
        for (index, (cap, meta)) in items.iter().enumerate() {
            list = list.child(render_capture_card(thread, index, cap, meta, cx));
        }
        Some(list.into_any_element())
    }

    #[cfg(feature = "automation")]
    pub(crate) fn capture_dump(&self, cx: &App) -> Vec<Value> {
        let Some(thread) = &self.thread_id else {
            return vec![];
        };
        let Some(store) = store(cx) else {
            return vec![];
        };
        store
            .for_thread(thread)
            .iter()
            .map(|(cap, meta)| {
                json!({
                    "captureId": cap.capture_id,
                    "kind": cap.kind,
                    "title": cap.display_title(),
                    "url": cap.source_url,
                    "quote": cap.quote,
                    "summary": cap.metadata_summary(),
                    "warnings": cap.warnings,
                    "hasImage": meta.image.is_some() || meta.thumbnail.is_some(),
                    "importing": meta.importing,
                    "importError": meta.import_error,
                    "expanded": meta.expanded,
                })
            })
            .collect()
    }
}

fn render_capture_actions(
    thread: &str,
    index: usize,
    cap: &PickCapture,
    meta: &PendingCaptureMeta,
    cx: &App,
) -> AnyElement {
    let secondary = style::secondary_label(cx);
    let accent = style::accent(cx);
    let has_image = meta.thumbnail.is_some() || meta.image.is_some();
    let has_quote = cap.quote.is_some();
    let mut actions = h_flex().flex_wrap().items_center().gap(px(8.)).pt(px(2.));
    if let Some(summary_text) = cap.metadata_summary() {
        let det_thread = thread.to_owned();
        let det_id = cap.capture_id.clone();
        let expanded = meta.expanded;
        let tip = if expanded {
            DETAILS_HIDE_TIP
        } else {
            DETAILS_SHOW_TIP
        };
        actions = actions.child(
            div()
                .id(("capture-details-toggle", index))
                .test_support()
                .role(Role::Button)
                .aria_label(tip)
                .cursor_pointer()
                .text_size(px(size::CHIP))
                .text_color(accent)
                .tip(tip)
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    toggle_details(&det_thread, &det_id, cx);
                })
                .child(if expanded {
                    "Hide details".to_owned()
                } else {
                    format!("Details · {summary_text}")
                }),
        );
    }
    if has_image && has_quote {
        let img_thread = thread.to_owned();
        let img_id = cap.capture_id.clone();
        actions = actions.child(
            div()
                .id(("capture-remove-image", index))
                .test_support()
                .role(Role::Button)
                .aria_label(REMOVE_IMAGE_TIP)
                .cursor_pointer()
                .text_size(px(size::CHIP))
                .text_color(secondary)
                .tip(REMOVE_IMAGE_TIP)
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    remove_screenshot(&img_thread, &img_id, cx);
                })
                .child("Remove image"),
        );
        let quote_thread = thread.to_owned();
        let quote_id = cap.capture_id.clone();
        actions = actions.child(
            div()
                .id(("capture-remove-quote", index))
                .test_support()
                .role(Role::Button)
                .aria_label(REMOVE_QUOTE_TIP)
                .cursor_pointer()
                .text_size(px(size::CHIP))
                .text_color(secondary)
                .tip(REMOVE_QUOTE_TIP)
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    remove_quote(&quote_thread, &quote_id, cx);
                })
                .child("Remove quote"),
        );
    }
    actions.into_any_element()
}

fn render_capture_card(
    thread: &str,
    index: usize,
    cap: &PickCapture,
    meta: &PendingCaptureMeta,
    cx: &App,
) -> AnyElement {
    let label = style::label_text(cx);
    let secondary = style::secondary_label(cx);
    let accent = style::accent(cx);
    let amber = style::needs_you(cx);
    let fill = style::chip_fill(cx);
    let hairline = style::hairline(cx);

    let details = if meta.expanded {
        cap.metadata_details()
    } else {
        vec![]
    };

    let discard_thread = thread.to_owned();
    let discard_id = cap.capture_id.clone();
    let discard_btn = div()
        .id(("capture-discard", index))
        .test_support()
        .role(Role::Button)
        .aria_label(DISCARD_TIP)
        .cursor_pointer()
        .flex_none()
        .p(px(2.))
        .rounded(px(4.))
        .tip(DISCARD_TIP)
        .on_click(move |_, _, cx| {
            cx.stop_propagation();
            discard(&discard_thread, &discard_id, cx);
        })
        .child(
            Icon::from(IconName::X)
                .with_size(px(11.))
                .text_color(secondary),
        );

    let mut header = h_flex().w_full().items_start().gap(px(8.)).when_some(
        meta.thumbnail.clone(),
        |row, thumb| {
            row.child(
                div()
                    .relative()
                    .flex_none()
                    .w(px(72.))
                    .h(px(48.))
                    .rounded(px(5.))
                    .overflow_hidden()
                    .border(gpui_kit::px(0.5))
                    .border_color(hairline)
                    .bg(style::primary_opacity(cx, 0.05))
                    .child(
                        img(crate::fit_picture::bytes_source(thumb))
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    )
                    .when(meta.importing, |box_el| {
                        box_el.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(style::primary_opacity(cx, 0.28))
                                .child(Spinner::new().color(label).with_size(px(12.))),
                        )
                    }),
            )
        },
    );

    let mut info = v_flex().flex_1().min_w_0().gap(px(2.)).child(
        div()
            .truncate()
            .text_size(px(size::LABEL))
            .font_weight(FontWeight::MEDIUM)
            .text_color(label)
            .child(cap.display_title()),
    );
    if !cap.source_url.is_empty() {
        info = info.child(
            div()
                .truncate()
                .text_size(px(size::CHIP))
                .text_color(secondary)
                .child(cap.source_url.clone()),
        );
    }
    info = info.child(render_capture_actions(thread, index, cap, meta, cx));
    header = header.child(info).child(discard_btn);

    let mut card = v_flex()
        .id(("page-capture-card", index))
        .test_support()
        .role(Role::Group)
        .aria_label(SharedString::from(format!(
            "Page capture: {}",
            cap.display_title()
        )))
        .w_full()
        .gap(px(5.))
        .px(px(10.))
        .py(px(8.))
        .rounded(px(8.))
        .bg(fill)
        .border(gpui_kit::px(0.5))
        .border_color(hairline)
        .child(header);

    if let Some(quote) = &cap.quote {
        card = card.child(
            div()
                .w_full()
                .pl(px(8.))
                .border_l_2()
                .border_color(accent.opacity(0.6))
                .text_size(px(size::CHIP))
                .text_color(label)
                .line_clamp(3)
                .overflow_hidden()
                .child(quote.clone()),
        );
    }

    if !details.is_empty() {
        let mut meta_box = v_flex()
            .w_full()
            .gap(px(2.))
            .px(px(8.))
            .py(px(5.))
            .rounded(px(5.))
            .bg(style::primary_opacity(cx, 0.04));
        for (k, v) in details {
            meta_box = meta_box.child(
                h_flex()
                    .gap(px(6.))
                    .text_size(px(size::CHIP))
                    .child(
                        div()
                            .flex_none()
                            .text_color(secondary)
                            .child(format!("{k}:")),
                    )
                    .child(div().min_w_0().truncate().text_color(label).child(v)),
            );
        }
        card = card.child(meta_box);
    }

    for note in cap.target_notes() {
        card = card.child(
            div()
                .text_size(px(size::CHIP))
                .text_color(secondary)
                .child(note),
        );
    }

    if let Some(err) = &meta.import_error {
        card = card.child(
            div()
                .text_size(px(size::CHIP))
                .text_color(amber)
                .child(err.clone()),
        );
    }

    if cap.has_secret_warning() {
        card = card.child(
            div()
                .text_size(px(size::CHIP))
                .text_color(amber)
                .child(SECRET_WARNING),
        );
    }

    card.into_any_element()
}
