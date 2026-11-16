// Pictures of your own on Threadbear's pages, a port of the Swift app's Looks,
// CustomizedPage and LookEditorSections: the Librarian, Scheduled and Projects
// pages' Customize panels (their picture, a background photo, where each part
// sits) and the main window's background picture. Each is a Settings > General
// > Look switch, off by default and kept in desktop-settings.json under the
// Swift app's UserDefaults names, never sent to the runner. Chosen pictures
// are copied into the data folder's `looks/`, scaled down, so moving or
// deleting the original doesn't break the page; replacing or removing one
// deletes its copy. The looks themselves (what's shown where) are core's
// `looks`, so both apps read one shape.
//
// Two Swift fixes have nothing to port here: the panel reads the saved look
// on every change rather than a copy that catches up a redraw later (a
// SwiftUI binding race that lost your own picture, 5b2e6d8e), and a clipped
// photo taking clicks above its box (79084074). GPUI draws what's held, and a
// picture with no handlers takes no clicks.
use crate::app::Threadbear;
use crate::a11y::A11yRole as _;
use crate::home_page::info_tip;
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use threadbear_core::looks::{BackdropFocus, LibrarianLook, Picture, PictureLook, PictureSize, Place, ProjectsLook, ScheduledLook};

/// The pages with a Customize panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Page {
    Librarian,
    Scheduled,
    Projects,
}

impl Page {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Page::Librarian => "librarian",
            Page::Scheduled => "scheduled",
            Page::Projects => "projects",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Page> {
        [Page::Librarian, Page::Scheduled, Page::Projects].into_iter().find(|p| p.name() == name)
    }
}

/// Where a background photo is drawn: the main window, or a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Slot {
    Main,
    Page(Page),
}

/// A page's look as it's kept: which page, and how it goes to and from the
/// settings file.
pub(crate) trait StoredLook: PictureLook + Clone + PartialEq + Default + 'static {
    const PAGE: Page;
    fn decode(v: &Value) -> Self;
    fn to_value(&self) -> Value;
    /// The choices the Picture section offers, and what Remove goes back to.
    fn pictures() -> &'static [Picture] {
        &[Picture::Bear, Picture::Own, Picture::None]
    }
    fn removed() -> Picture {
        Picture::Bear
    }
}

impl StoredLook for LibrarianLook {
    const PAGE: Page = Page::Librarian;
    fn decode(v: &Value) -> Self {
        LibrarianLook::decode(v)
    }
    fn to_value(&self) -> Value {
        LibrarianLook::to_value(self)
    }
}

impl StoredLook for ScheduledLook {
    const PAGE: Page = Page::Scheduled;
    fn decode(v: &Value) -> Self {
        ScheduledLook::decode(v)
    }
    fn to_value(&self) -> Value {
        ScheduledLook::to_value(self)
    }
}

impl StoredLook for ProjectsLook {
    const PAGE: Page = Page::Projects;
    fn decode(v: &Value) -> Self {
        ProjectsLook::decode(v)
    }
    fn to_value(&self) -> Value {
        ProjectsLook::to_value(self)
    }
    // The page never had a bear, so it's your picture or none.
    fn pictures() -> &'static [Picture] {
        &ProjectsLook::PICTURES
    }
    fn removed() -> Picture {
        Picture::None
    }
}

/// What a page's background or picture well is: the picture on the page, or
/// the photo behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Well {
    Picture,
    Background,
}

impl Well {
    fn key(self) -> &'static str {
        match self {
            Well::Picture => "picture",
            Well::Background => "background",
        }
    }
}

/// What the Look parts of the app hold while they're showing.
#[derive(Default)]
pub(crate) struct LooksState {
    /// The page whose Customize panel is open beside it.
    pub(crate) editing: Option<Page>,
    /// Why the last picture couldn't be used, by "<slot>/<well>".
    problems: HashMap<String, String>,
    /// Pictures being copied in, by the same key: their buttons spin.
    importing: HashSet<String>,
    /// The size each background was last drawn at, in points before zoom,
    /// for Position's preview (`BackdropSizes`).
    sizes: Rc<RefCell<HashMap<Slot, (f32, f32)>>>,
    /// Where a Position drag started: the focus then, and the pointer.
    drag: Option<(Slot, BackdropFocus, Point<Pixels>)>,
    /// The well a picture is being dragged over.
    pub(crate) drop_over: Option<String>,
    /// Each background's Fade slider.
    fades: HashMap<Slot, (Entity<SliderState>, Subscription)>,
    /// The panel's own scroll, a page at a time.
    pub(crate) panel_scroll: HashMap<Page, ScrollHandle>,
}

fn key(slot: Slot, well: Well) -> String {
    let slot = match slot {
        Slot::Main => "main",
        Slot::Page(p) => p.name(),
    };
    format!("{slot}/{}", well.key())
}

/// In the runner's data folder, so each profile's pictures are its own. A
/// mock's or a dev runner's data folder is a temporary one, so a scripted
/// app's pictures never land among Mike's.
pub(crate) fn folder() -> PathBuf {
    threadbear_client::config::RunnerConfig::from_env().home.join("looks")
}

/// A picture in `looks/` by its name, never anything outside it.
pub(crate) fn path(name: &str) -> Option<PathBuf> {
    (!name.is_empty() && !name.contains('/') && !name.contains('\\') && name != "." && name != "..").then(|| folder().join(name))
}

fn remove(name: &str) {
    if let Some(path) = path(name) {
        DIMS.with(|d| d.borrow_mut().remove(name));
        let _ = std::fs::remove_file(path);
    }
}

thread_local! {
    /// Each picture's size in pixels, read once from its header. Names are
    /// never reused (each copy gets a new one), so nothing here goes stale.
    static DIMS: RefCell<HashMap<String, Option<(f32, f32)>>> = RefCell::new(HashMap::new());
}

/// A picture's size, or None when it's gone or unreadable.
pub(crate) fn dims(name: &str) -> Option<(f32, f32)> {
    if let Some(known) = DIMS.with(|d| d.borrow().get(name).copied()) {
        return known;
    }
    let read = path(name).and_then(|p| image::image_dimensions(p).ok()).map(|(w, h)| (w as f32, h as f32));
    DIMS.with(|d| d.borrow_mut().insert(name.to_owned(), read));
    read
}

/// Copies a picture in, scaled so its longest side is at most `max_side`
/// pixels: a picture with see-through parts stays a PNG, a photo becomes a
/// JPEG. With `animated` (backgrounds), an animated GIF is copied as it is,
/// so it plays. Runs off the main thread.
pub(crate) fn import(from: &Path, max_side: u32, prefix: &str, animated: bool) -> Result<String, String> {
    const UNREADABLE: &str = "That file isn't a picture Threadbear can read.";
    let bytes = std::fs::read(from).map_err(|_| UNREADABLE.to_owned())?;
    let format = image::guess_format(&bytes).map_err(|_| UNREADABLE.to_owned())?;
    if animated && format == image::ImageFormat::Gif && gif_frames(&bytes) > 1 {
        return save(&bytes, "gif", prefix);
    }
    let picture = image::load_from_memory_with_format(&bytes, format).map_err(|_| UNREADABLE.to_owned())?;
    let (w, h) = (picture.width(), picture.height());
    let picture = if w.max(h) > max_side { picture.resize(max_side, max_side, image::imageops::FilterType::Lanczos3) } else { picture };
    let mut out = Vec::new();
    let ext = if picture.color().has_alpha() {
        picture.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).map_err(|_| UNREADABLE.to_owned())?;
        "png"
    } else {
        let rgb = picture.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&rgb).map_err(|_| UNREADABLE.to_owned())?;
        "jpg"
    };
    save(&out, ext, prefix)
}

/// How many frames a GIF has, stopping at two: one is a still.
fn gif_frames(bytes: &[u8]) -> usize {
    use image::AnimationDecoder as _;
    image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes)).map(|d| d.into_frames().take(2).filter(Result::is_ok).count()).unwrap_or(0)
}

fn save(bytes: &[u8], ext: &str, prefix: &str) -> Result<String, String> {
    let dir = folder();
    threadbear_core::private_file::dir(&dir).map_err(|e| format!("Couldn't keep a copy of the picture: {e}"))?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let name = format!("{prefix}-{:08x}.{ext}", (stamp as u64 ^ (stamp >> 64) as u64 ^ u64::from(std::process::id())) as u32);
    threadbear_core::private_file::write(&dir.join(&name), bytes).map_err(|e| format!("Couldn't keep a copy of the picture: {e}"))?;
    Ok(name)
}

/// The pictures a look no longer uses go with it. Each page's files are its
/// own (every copy gets a new name), so one page never deletes another's.
fn remove_unused(old: &impl PictureLook, new: &impl PictureLook) {
    let (o, n) = (old.pic(), new.pic());
    for file in [o.picture_file.as_deref(), o.background.as_deref()].into_iter().flatten() {
        if Some(file) != n.picture_file.as_deref() && Some(file) != n.background.as_deref() {
            remove(file);
        }
    }
}

/// The main window's background picture, as kept.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MainBackground {
    pub(crate) on: bool,
    pub(crate) file: Option<String>,
    pub(crate) dim: f64,
    pub(crate) focus: BackdropFocus,
    pub(crate) everywhere: bool,
}

pub(crate) const DEFAULT_DIM: f64 = 0.55;

impl Threadbear {
    pub(crate) fn customizable(&self, page: Page) -> bool {
        let s = &self.settings.saved;
        match page {
            Page::Librarian => s.librarian_look_on,
            Page::Scheduled => s.scheduled_look_on,
            Page::Projects => s.projects_look_on,
        }
        .unwrap_or(false)
    }

    pub(crate) fn set_customizable(&mut self, page: Page, on: bool, cx: &mut Context<Self>) {
        let s = &mut self.settings.saved;
        *match page {
            Page::Librarian => &mut s.librarian_look_on,
            Page::Scheduled => &mut s.scheduled_look_on,
            Page::Projects => &mut s.projects_look_on,
        } = Some(on);
        if !on && self.looks.editing == Some(page) {
            self.looks.editing = None;
        }
        self.save_settings_soon(cx);
        cx.notify();
    }

    fn look_slot(&mut self, page: Page) -> &mut Option<Value> {
        let s = &mut self.settings.saved;
        match page {
            Page::Librarian => &mut s.librarian_look,
            Page::Scheduled => &mut s.scheduled_look,
            Page::Projects => &mut s.projects_look,
        }
    }

    /// The page's look as saved, or the standard one.
    pub(crate) fn look<L: StoredLook>(&self) -> L {
        let s = &self.settings.saved;
        let v = match L::PAGE {
            Page::Librarian => &s.librarian_look,
            Page::Scheduled => &s.scheduled_look,
            Page::Projects => &s.projects_look,
        };
        v.as_ref().map(L::decode).unwrap_or_default()
    }

    /// Saves it, and deletes the copies of any pictures it no longer uses.
    pub(crate) fn save_look<L: StoredLook>(&mut self, new: L, cx: &mut Context<Self>) {
        let old: L = self.look();
        if old == new {
            return;
        }
        *self.look_slot(L::PAGE) = Some(new.to_value());
        remove_unused(&old, &new);
        if L::PAGE == Page::Librarian {
            let (mut old_value, mut new_value) = (old.to_value(), new.to_value());
            old_value["profileButton"] = Value::Null;
            new_value["profileButton"] = Value::Null;
            if old_value != new_value { self.librarian_look_changed(cx); }
        }
        self.save_settings_soon(cx);
        cx.notify();
    }

    /// One change to the saved look, read fresh, so a run of changes never
    /// undoes an earlier one.
    pub(crate) fn edit_look<L: StoredLook>(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut L)) {
        let mut look: L = self.look();
        change(&mut look);
        self.save_look(look, cx);
    }

    pub(crate) fn main_background(&self) -> MainBackground {
        let s = &self.settings.saved;
        MainBackground {
            on: s.main_background_on.unwrap_or(false),
            file: s.main_background.clone().filter(|f| !f.is_empty()),
            dim: s.main_background_dim.unwrap_or(DEFAULT_DIM).clamp(0., 1.),
            focus: BackdropFocus::new(s.main_background_x.unwrap_or(BackdropFocus::BOTTOM.x), s.main_background_y.unwrap_or(BackdropFocus::BOTTOM.y)),
            everywhere: s.main_background_everywhere.unwrap_or(false),
        }
    }

    pub(crate) fn edit_main_background(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut MainBackground)) {
        let old = self.main_background();
        let mut new = old.clone();
        change(&mut new);
        if old == new {
            return;
        }
        if let Some(file) = old.file.as_deref().filter(|f| Some(*f) != new.file.as_deref()) {
            remove(file);
        }
        let s = &mut self.settings.saved;
        s.main_background_on = Some(new.on);
        s.main_background = new.file;
        s.main_background_dim = Some(new.dim);
        s.main_background_x = Some(new.focus.x);
        s.main_background_y = Some(new.focus.y);
        s.main_background_everywhere = Some(new.everywhere);
        self.save_settings_soon(cx);
        cx.notify();
    }

    /// Choose… and Replace…: a file picker for one picture.
    pub(crate) fn choose_look_picture(&mut self, slot: Slot, well: Well, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = if well == Well::Picture { "Use Picture" } else { "Use as Background" };
        let picked = crate::picture_picker::choose(window, prompt, cx);
        cx.spawn(async move |this, cx| {
            if let Some(path) = picked.await {
                this.update(cx, |this, cx| this.use_look_picture(slot, well, path, cx)).ok();
            }
        }).detach();
    }

    /// A picture dropped on a well: a file from Finder, or one dragged out
    /// of Photos or a browser, which the composer's drop reader turns into
    /// a file first.
    pub(crate) fn drop_look_picture(&mut self, slot: Slot, well: Well, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.looks.drop_over = None;
        cx.notify();
        let paths = crate::composer_media::read(true).unwrap_or(paths);
        let Some(first) = paths.into_iter().next() else { return };
        if !self.looks.importing.insert(key(slot, well)) { return; }
        cx.spawn(async move |this, cx| {
            let grace = cx.background_executor().timer(std::time::Duration::from_millis(500));
            let ready = crate::composer_media::ready(&first, grace);
            let timeout = cx.background_executor().timer(std::time::Duration::from_secs(60));
            futures::pin_mut!(ready, timeout);
            match futures::future::select(ready, timeout).await {
                futures::future::Either::Left((Ok(prepared), _)) => {
                    let Some(path) = prepared.paths.first().cloned() else { return };
                    // Copied into looks/ before the dropped copy is let go.
                    let made = Self::import_off_main(&path, slot, well, cx).await;
                    drop(prepared);
                    this.update(cx, |this, cx| this.picture_imported(slot, well, made, cx)).ok();
                }
                futures::future::Either::Left((Err(e), _)) => {
                    this.update(cx, |this, cx| this.picture_imported(slot, well, Err(e), cx)).ok();
                }
                futures::future::Either::Right(_) => {
                    this.update(cx, |this, cx| this.picture_imported(slot, well, Err("The picture took too long to arrive.".into()), cx)).ok();
                }
            }
        })
        .detach();
    }

    async fn import_off_main(path: &Path, slot: Slot, well: Well, cx: &mut AsyncApp) -> Result<String, String> {
        let (max, prefix) = match (well, slot) {
            (Well::Picture, _) => (1200, "picture"),
            (Well::Background, Slot::Main) => (2880, "window"),
            (Well::Background, _) => (2880, "background"),
        };
        let path = path.to_owned();
        cx.background_executor().spawn(async move { import(&path, max, prefix, well == Well::Background) }).await
    }

    /// A chosen or dropped picture, copied in and put on the page.
    pub(crate) fn use_look_picture(&mut self, slot: Slot, well: Well, path: PathBuf, cx: &mut Context<Self>) {
        let k = key(slot, well);
        if !self.looks.importing.insert(k) {
            return;
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let made = Self::import_off_main(&path, slot, well, cx).await;
            this.update(cx, |this, cx| this.picture_imported(slot, well, made, cx)).ok();
        })
        .detach();
    }

    fn picture_imported(&mut self, slot: Slot, well: Well, made: Result<String, String>, cx: &mut Context<Self>) {
        let k = key(slot, well);
        self.looks.importing.remove(&k);
        let name = match made {
            Ok(name) => name,
            Err(problem) => {
                self.looks.problems.insert(k, problem);
                cx.notify();
                return;
            }
        };
        self.looks.problems.remove(&k);
        // One change, so the new file and the choice land together, and a
        // new photo starts on the bottom edge rather than where the last was.
        match slot {
            Slot::Main => self.edit_main_background(cx, |b| {
                b.file = Some(name);
                b.focus = BackdropFocus::BOTTOM;
            }),
            Slot::Page(Page::Librarian) => self.edit_look::<LibrarianLook>(cx, |l| set_picture(l, well, name)),
            Slot::Page(Page::Scheduled) => self.edit_look::<ScheduledLook>(cx, |l| set_picture(l, well, name)),
            Slot::Page(Page::Projects) => self.edit_look::<ProjectsLook>(cx, |l| set_picture(l, well, name)),
        }
        cx.notify();
    }

    /// The focus a background has in a slot, and a change to it.
    fn backdrop_focus(&self, slot: Slot) -> BackdropFocus {
        match slot {
            Slot::Main => self.main_background().focus,
            Slot::Page(Page::Librarian) => self.look::<LibrarianLook>().pic().background_focus,
            Slot::Page(Page::Scheduled) => self.look::<ScheduledLook>().pic().background_focus,
            Slot::Page(Page::Projects) => self.look::<ProjectsLook>().pic().background_focus,
        }
    }

    fn set_backdrop_focus(&mut self, slot: Slot, focus: BackdropFocus, cx: &mut Context<Self>) {
        match slot {
            Slot::Main => self.edit_main_background(cx, |b| b.focus = focus),
            Slot::Page(Page::Librarian) => self.edit_look::<LibrarianLook>(cx, |l| l.pic_mut().background_focus = focus),
            Slot::Page(Page::Scheduled) => self.edit_look::<ScheduledLook>(cx, |l| l.pic_mut().background_focus = focus),
            Slot::Page(Page::Projects) => self.edit_look::<ProjectsLook>(cx, |l| l.pic_mut().background_focus = focus),
        }
    }

    fn set_backdrop_dim(&mut self, slot: Slot, dim: f64, cx: &mut Context<Self>) {
        match slot {
            Slot::Main => self.edit_main_background(cx, |b| b.dim = dim),
            Slot::Page(Page::Librarian) => self.edit_look::<LibrarianLook>(cx, |l| l.pic_mut().background_dim = dim),
            Slot::Page(Page::Scheduled) => self.edit_look::<ScheduledLook>(cx, |l| l.pic_mut().background_dim = dim),
            Slot::Page(Page::Projects) => self.edit_look::<ProjectsLook>(cx, |l| l.pic_mut().background_dim = dim),
        }
    }

    // Drawing.

    /// A picture filling the space behind a page, with the window's own
    /// colour laid over it (`dim`) so the text on top stays readable in light
    /// and dark. Cropped around `focus`: bottom centre unless moved, so the
    /// photo's bottom edge stays on the window's as it resizes. An animated
    /// GIF plays, and holds still with Reduce Motion on (GPUI's own picture
    /// does both). Nothing when the picture's gone; it takes no clicks.
    pub(crate) fn render_backdrop(&self, slot: Slot, file: &str, dim: f64, focus: BackdropFocus, cx: &mut Context<Self>) -> Option<AnyElement> {
        let path = path(file)?;
        let image = dims(file)?;
        let sizes = self.looks.sizes.clone();
        let drawn = sizes.borrow().get(&slot).copied();
        let (x, y, w, h) = drawn.map_or((0., 0., 0., 0.), |space| focus.frame(image, space));
        let notify = cx.entity().downgrade();
        let wash = cx.theme().background.opacity(dim as f32);
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .overflow_hidden()
                .when(drawn.is_some(), |d| d.child(div().id("look-background-image").test_support().role(Role::Image).aria_label("Page background photo").absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).child(img(path).size_full().object_fit(ObjectFit::Fill))))
                .child(div().absolute().top_0().left_0().size_full().bg(wash))
                // The size it's drawn at, for the next frame and for Position.
                .child(
                    canvas(
                        move |bounds, _, cx| {
                            let one = style::zoomed(1.);
                            let now = (f32::from(bounds.size.width) / one, f32::from(bounds.size.height) / one);
                            let was = sizes.borrow().get(&slot).copied();
                            if now.0 > 1. && now.1 > 1. && was.is_none_or(|w| (w.0 - now.0).abs() > 0.5 || (w.1 - now.1).abs() > 0.5) {
                                sizes.borrow_mut().insert(slot, now);
                                let notify = notify.clone();
                                cx.defer(move |cx| { notify.update(cx, |_, cx| cx.notify()).ok(); });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
                .into_any_element(),
        )
    }

    /// The frame every customized page shares (`CustomizedPage`): its parts
    /// in a scrolling column over its background photo, Customize at the top
    /// right, and the page's panel beside it, so each change shows as it's
    /// made. With `pins_places`, top, middle and bottom are where parts sit
    /// in the window (middle in the space between, bottom on its bottom
    /// edge), and a page too tall for the window stacks them and scrolls;
    /// without, they're only the order parts stack in, as the Librarian page
    /// has always had them.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_customized_page(
        &mut self,
        page: Page,
        backdrop: Option<AnyElement>,
        zones: [Vec<AnyElement>; 3],
        pins_places: bool,
        max_width: f32,
        customize_tip: &'static str,
        buttons: Vec<AnyElement>,
        editor: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editing = self.looks.editing == Some(page);
        let secondary = style::secondary_text(cx);
        let gap = 32.;
        let filled: Vec<bool> = zones.iter().map(|z| !z.is_empty()).collect();
        let column: AnyElement = if filled.iter().all(|f| !f) {
            div().text_size(px(style::size::LABEL)).line_height(px(16.)).text_color(secondary).child("Everything on this page is hidden. Customize puts it back.").into_any_element()
        } else if pins_places {
            let [top, middle, bottom] = zones;
            let any_after_top = filled[1] || filled[2];
            v_flex()
                .w_full()
                .flex_1()
                .when(filled[0], |c| c.child(v_flex().w_full().gap(px(32.)).children(top)))
                // The gap only between places that have something, so an
                // empty one leaves no hole when the page scrolls.
                .child(div().flex_1().min_h(px(if filled[0] && any_after_top { gap } else { 0. })))
                .when(filled[1], |c| c.child(v_flex().w_full().gap(px(32.)).children(middle)))
                .child(div().flex_1().min_h(px(if filled[1] && filled[2] { gap } else { 0. })))
                .when(filled[2], |c| c.child(v_flex().w_full().gap(px(32.)).children(bottom)))
                .into_any_element()
        } else {
            v_flex().w_full().gap(px(gap)).children(zones.into_iter().flatten()).into_any_element()
        };
        let customize = (!editing).then(|| {
            kit_button(format!("customize-{}", page.name()), "Customize", Some("paintbrush"), customize_tip, false, false, cx.listener(move |this, _, _, cx| {
                this.looks.editing = Some(page);
                if page == Page::Librarian { this.load_profile_page(cx); }
                cx.notify();
            }))
        });
        let area = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .children(backdrop)
            .child(
                div().id(SharedString::from(format!("{}-home", page.name()))).size_full().overflow_y_scroll().child(
                    h_flex().w_full().min_h_full().justify_center().child(
                        v_flex()
                            .w_full()
                            .min_h_full()
                            .max_w(px(max_width + 64.))
                            .px(px(32.))
                            // Clear of the Customize button, which a wide part
                            // (the numbers) at the top would otherwise run under.
                            .pt(px(56.))
                            .pb(px(48.))
                            .child(column),
                    ),
                ),
            )
            .child(h_flex().absolute().top(px(14.)).right(px(14.)).gap(px(8.)).children(buttons).children(customize));
        let mut row = h_flex().size_full().child(area);
        if let Some(editor) = editor.filter(|_| editing) {
            row = row.child(div().h_full().w(gpui_kit::px(1.)).bg(style::hairline(cx))).child(editor);
        }
        row.into_any_element()
    }

    /// The panel's frame: Customize and Done at the top, its sections under
    /// it in plain cards, scrolling.
    pub(crate) fn render_look_editor(&mut self, page: Page, sections: Vec<AnyElement>, cx: &mut Context<Self>) -> AnyElement {
        let scroll = self.looks.panel_scroll.entry(page).or_default().clone();
        v_flex()
            .id(SharedString::from(format!("customize-panel-{}", page.name())))
            .flex_none()
            .w(px(320.))
            .h_full()
            .bg(cx.theme().background)
            // Escape closes it, as Done's `cancelAction` does.
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" {
                    this.looks.editing = None;
                    cx.notify();
                }
            }))
            .child(
                h_flex()
                    .flex_none()
                    .px(px(16.))
                    .py(px(12.))
                    .items_center()
                    .child(div().flex_1().text_size(px(15.)).line_height(px(19.)).font_semibold().child("Customize"))
                    .child(kit_button(format!("customize-done-{}", page.name()), "Done", None, "Close Customize. Changes are already saved", false, false, cx.listener(|this, _, _, cx| {
                        this.looks.editing = None;
                        cx.notify();
                    }))),
            )
            .child(div().flex_none().h(gpui_kit::px(1.)).bg(style::hairline(cx)))
            .child(div().id(SharedString::from(format!("customize-scroll-{}", page.name()))).flex_1().min_h_0().overflow_y_scroll().track_scroll(&scroll).child(v_flex().p(px(14.)).gap(px(18.)).children(sections)))
            .into_any_element()
    }

    /// The Picture section (`LookPictureSection`): the bear, a picture of
    /// your own or none, and its size. Plain buttons, so a click always
    /// changes it (the Swift app's segmented control took clicks and did
    /// nothing, 59b51fa1).
    pub(crate) fn render_picture_section<L: StoredLook>(&mut self, bear_tip: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let look: L = self.look();
        let pic = look.pic().clone();
        let page = L::PAGE;
        let slot = Slot::Page(page);
        let k = key(slot, Well::Picture);
        let mut rows = vec![];
        let options: Vec<(Picture, &'static str)> = L::pictures()
            .iter()
            .map(|p| {
                let tip = match p {
                    Picture::Bear => bear_tip,
                    Picture::Own => "Show a picture of your own",
                    Picture::None => "No picture on the page",
                };
                (*p, tip)
            })
            .collect();
        let me = cx.entity().downgrade();
        rows.push(segmented(format!("{}-picture", page.name()), options.iter().map(|(p, tip)| (p.title(), *tip, *p == pic.picture)).collect(), move |i, window, cx| {
            let chosen = options[i].0;
            me.update(cx, |this, cx| {
                // Your picture with none chosen yet asks for one straight away.
                if chosen == Picture::Own && this.look::<L>().pic().picture_file.is_none() {
                    this.choose_look_picture(slot, Well::Picture, window, cx);
                } else {
                    this.edit_look::<L>(cx, |l| l.pic_mut().picture = chosen);
                }
            })
            .ok();
        }, cx));
        if pic.picture == Picture::Own {
            rows.push(self.picture_well(slot, Well::Picture, pic.picture_file.as_deref(), "Drop a picture here", false, cx));
            let mut buttons = h_flex().gap(px(8.)).child(kit_button(format!("{}-picture-choose", page.name()), if pic.picture_file.is_none() { "Choose…" } else { "Replace…" }, None, "Pick a picture from your Mac. A copy is kept, so the original can move", false, self.looks.importing.contains(&k), cx.listener(move |this, _, window, cx| this.choose_look_picture(slot, Well::Picture, window, cx))));
            if pic.picture_file.is_some() {
                let tip = if L::removed() == Picture::Bear { "Go back to the bear and delete the copy" } else { "Take the picture off the page and delete the copy" };
                buttons = buttons.child(kit_button(format!("{}-picture-remove", page.name()), "Remove", None, tip, false, false, cx.listener(move |this, _, _, cx| {
                    this.edit_look::<L>(cx, |l| {
                        l.pic_mut().picture_file = None;
                        l.pic_mut().picture = L::removed();
                    })
                })));
            }
            rows.push(buttons.into_any_element());
        }
        if pic.picture != Picture::None {
            let me = cx.entity().downgrade();
            let menu = choice_menu(format!("{}-picture-size", page.name()), pic.picture_size.title(), "How big the picture is on the page", PictureSize::ALL.iter().map(|s| (s.title().to_owned(), *s == pic.picture_size)).collect(), false, move |i, cx| {
                let size = PictureSize::ALL[i];
                me.update(cx, |this, cx| this.edit_look::<L>(cx, |l| l.pic_mut().picture_size = size)).ok();
            });
            rows.push(labeled("Size", menu, cx));
        }
        if let Some(problem) = self.looks.problems.get(&k) {
            rows.push(problem_line(problem, cx));
        }
        panel_section("Picture", None, rows, cx)
    }

    /// The Background section (`LookBackgroundSection`): a photo behind the
    /// page, where it sits when cropped, and how faded it is.
    pub(crate) fn render_background_section<L: StoredLook>(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let look: L = self.look();
        let pic = look.pic().clone();
        let slot = Slot::Page(L::PAGE);
        let rows = self.background_rows(slot, pic.background.as_deref(), pic.background_dim, pic.background_focus, true, cx);
        panel_section("Background", None, rows, cx)
    }

    /// A background's rows, for a page's panel and for Settings: the well (in
    /// the panel), Choose and Remove, Position, Fade, and what went wrong.
    pub(crate) fn background_rows(&mut self, slot: Slot, file: Option<&str>, dim: f64, focus: BackdropFocus, well: bool, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let k = key(slot, Well::Background);
        let id = match slot {
            Slot::Main => "main".to_owned(),
            Slot::Page(p) => p.name().to_owned(),
        };
        let mut rows = vec![];
        if well {
            rows.push(self.picture_well(slot, Well::Background, file, "Drop a photo here", true, cx));
        }
        let mut buttons = h_flex().gap(px(8.)).child(kit_button(format!("{id}-background-choose"), if file.is_none() { "Choose…" } else { "Replace…" }, None, "Pick a photo or an animated GIF to fill the page behind everything", false, self.looks.importing.contains(&k), cx.listener(move |this, _, window, cx| this.choose_look_picture(slot, Well::Background, window, cx))));
        if file.is_some() {
            buttons = buttons.child(kit_button(format!("{id}-background-remove"), "Remove", None, "Take the background away and delete the copy", false, false, cx.listener(move |this, _, _, cx| match slot {
                Slot::Main => this.edit_main_background(cx, |b| b.file = None),
                Slot::Page(Page::Librarian) => this.edit_look::<LibrarianLook>(cx, |l| l.pic_mut().background = None),
                Slot::Page(Page::Scheduled) => this.edit_look::<ScheduledLook>(cx, |l| l.pic_mut().background = None),
                Slot::Page(Page::Projects) => this.edit_look::<ProjectsLook>(cx, |l| l.pic_mut().background = None),
            })));
        }
        rows.push(if well { buttons.into_any_element() } else { labeled("Picture", buttons.into_any_element(), cx) });
        if let Some(file) = file.filter(|f| dims(f).is_some()) {
            let positioner = self.render_positioner(slot, file, focus, if well { 200. } else { 240. }, if well { 130. } else { 140. }, cx);
            rows.push(labeled_top("Position", positioner, cx));
            let fade = self.fade_slider(slot, dim, cx);
            rows.push(labeled("Fade", fade, cx));
        }
        if let Some(problem) = self.looks.problems.get(&k) {
            rows.push(problem_line(problem, cx));
        }
        rows
    }

    fn fade_slider(&mut self, slot: Slot, dim: f64, cx: &mut Context<Self>) -> AnyElement {
        let state = match self.looks.fades.get(&slot) {
            Some((state, _)) => state.clone(),
            None => {
                let state = cx.new(|_| SliderState::new().min(0.).max(0.9).step(0.01).default_value(dim as f32));
                let sub = cx.subscribe(&state, move |this, _, ev: &SliderEvent, cx| {
                    let v = match ev {
                        SliderEvent::Change(v) | SliderEvent::Release(v) => v.start(),
                    };
                    this.set_backdrop_dim(slot, f64::from(v), cx);
                });
                self.looks.fades.insert(slot, (state.clone(), sub));
                state
            }
        };
        // Reset or another window changed it: the slider follows.
        let shown = f64::from(state.read(cx).value().start());
        if (shown - dim).abs() > 0.005 && !cx.has_active_drag() {
            let target = dim as f32;
            let state = state.clone();
            cx.defer(move |cx| {
                if let Some(window) = cx.active_window() {
                    window.update(cx, |_, window, cx| state.update(cx, |s, cx| s.set_value(target, window, cx))).ok();
                }
            });
        }
        let tip = "How much the page's own colour covers the photo, so text stays easy to read in light and dark";
        div().id(SharedString::from(format!("{}-fade", slot_name(slot)))).test_support().a11y_role(Role::GenericContainer).aria_label(tip).w(px(180.)).tip(tip).child(Slider::new(&state).horizontal()).into_any_element()
    }

    /// Position (`BackdropPositioner`): the photo in a frame the shape of
    /// where it's drawn, to drag to choose what shows, with Bottom to put it
    /// back on the window's bottom edge. The page behind follows as you
    /// drag, so you see it for real too.
    fn render_positioner(&mut self, slot: Slot, file: &str, focus: BackdropFocus, max_w: f32, max_h: f32, cx: &mut Context<Self>) -> AnyElement {
        let Some(image) = dims(file) else { return div().into_any_element() };
        let space = self.looks.sizes.borrow().get(&slot).copied().unwrap_or((1600., 1000.));
        let ratio = (space.0 / space.1.max(1.)).clamp(0.4, 3.);
        // As wide as it can be without growing past max_h, so a tall
        // window's shape doesn't make a tower.
        let width = max_w.min(max_h * ratio);
        let height = width / ratio;
        let preview = (width, height);
        let (x, y, w, h) = focus.frame(image, preview);
        let moves = w - width > 0.5 || h - height > 0.5;
        let dragging = self.looks.drag.is_some_and(|(s, _, _)| s == slot) && cx.has_active_drag();
        let edge = if dragging { style::control_accent(cx) } else { style::primary_opacity(cx, 0.2) };
        let tip = if moves { "Drag the photo to choose which part shows in the window" } else { "The photo fits the window's shape exactly, so there's nothing to move" };
        let path = path(file);
        let frame = div()
            .id(SharedString::from(format!("{}-position", slot_name(slot))))
            .test_support()
            .role(Role::Image)
            .aria_label(tip)
            .tip(tip)
            .relative()
            .w(px(width))
            .h(px(height))
            .rounded(px(6.))
            .overflow_hidden()
            .border(gpui_kit::px(if dragging { 2. } else { 1. }))
            .border_color(edge)
            .when(moves, |d| d.cursor(if dragging { CursorStyle::ClosedHand } else { CursorStyle::OpenHand }))
            .when_some(path, |d, path| d.child(img(path).absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).object_fit(ObjectFit::Fill)))
            .when(moves, |d| {
                d.on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    let start = this.backdrop_focus(slot);
                    this.looks.drag = Some((slot, start, ev.position));
                    cx.notify();
                }))
                .on_drag(PositionDrag(slot), |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(cx.listener(move |this, ev: &DragMoveEvent<PositionDrag>, _, cx| {
                    let PositionDrag(dragged) = *ev.drag(cx);
                    let Some((at, start, from)) = this.looks.drag.filter(|(s, _, _)| *s == dragged && dragged == slot) else { return };
                    let one = style::zoomed(1.);
                    let by = (f32::from(ev.event.position.x - from.x) / one, f32::from(ev.event.position.y - from.y) / one);
                    let next = BackdropFocus::dragged(start, by, image, preview);
                    if next != this.backdrop_focus(at) {
                        this.set_backdrop_focus(at, next, cx);
                    }
                }))
            });
        let bottom = kit_button(format!("{}-position-bottom", slot_name(slot)), "Bottom", None, "Put the photo back with its bottom edge on the window's bottom edge", focus == BackdropFocus::BOTTOM, false, cx.listener(move |this, _, _, cx| this.set_backdrop_focus(slot, BackdropFocus::BOTTOM, cx)));
        v_flex()
            .items_end()
            .gap(px(6.))
            .child(frame)
            .child(h_flex().gap(px(8.)).items_center().child(div().whitespace_nowrap().text_size(px(style::size::LABEL)).text_color(style::secondary_text(cx)).child(if moves { "Drag to position" } else { "Fits the window" })).child(bottom))
            .into_any_element()
    }

    /// A picture's spot in the panel (`PictureWell`): the picture, or a
    /// dashed box to drop one on.
    fn picture_well(&mut self, slot: Slot, well: Well, file: Option<&str>, empty: &'static str, fill: bool, cx: &mut Context<Self>) -> AnyElement {
        let k = key(slot, well);
        let over = self.looks.drop_over.as_deref() == Some(k.as_str());
        let path = file.and_then(path).filter(|p| p.exists());
        let has = path.is_some();
        let edge = if over { style::control_accent(cx) } else if has { gpui_kit::transparent_black() } else { style::primary_opacity(cx, 0.2) };
        let (enter, leave) = (k.clone(), k.clone());
        let tip = "Drop a picture here from Finder, Photos or a browser";
        let mut d = div()
            .id(SharedString::from(format!("{k}-well")))
            .test_support()
            .role(Role::Image)
            .aria_label(tip)
            .tip(tip)
            .w_full()
            .rounded(px(8.))
            .overflow_hidden()
            .border(gpui_kit::px(if over { 2. } else { 1. }))
            .border_color(edge)
            .when(!has && !over, |d| d.border_dashed())
            .drag_over::<ExternalPaths>(move |s, _, _, _| s.border_color(gpui_kit::blue()))
            .on_drag_move(cx.listener(move |this, _: &DragMoveEvent<ExternalPaths>, _, cx| {
                if this.looks.drop_over.as_deref() != Some(enter.as_str()) {
                    this.looks.drop_over = Some(enter.clone());
                    cx.notify();
                }
            }))
            .on_drop(cx.listener(move |this, paths: &ExternalPaths, _, cx| {
                let _ = &leave;
                this.drop_look_picture(slot, well, paths.paths().to_vec(), cx);
            }));
        d = match path {
            Some(path) => d.h(px(120.)).child(img(path).size_full().object_fit(if fill { ObjectFit::Cover } else { ObjectFit::Contain })),
            None => d.h(px(64.)).flex().items_center().justify_center().text_size(px(style::size::LABEL)).text_color(style::secondary_text(cx)).child(empty),
        };
        d.into_any_element()
    }
}

/// The marker a Position drag carries.
#[derive(Clone, Copy)]
pub(crate) struct PositionDrag(Slot);

impl Render for PositionDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

fn slot_name(slot: Slot) -> &'static str {
    match slot {
        Slot::Main => "main",
        Slot::Page(p) => p.name(),
    }
}

fn set_picture<L: PictureLook>(look: &mut L, well: Well, name: String) {
    match well {
        Well::Picture => {
            look.pic_mut().picture_file = Some(name);
            look.pic_mut().picture = Picture::Own;
            look.show_picture();
        }
        Well::Background => {
            look.pic_mut().background = Some(name);
            look.pic_mut().background_focus = BackdropFocus::BOTTOM;
        }
    }
}

// The panel's pieces (`PanelFormStyle`): plain cards, so where a control is
// drawn is where it's clicked.

/// A section: its heading (with an ⓘ when it has more to say), then its rows
/// in a rounded card with a line between each two.
pub(crate) fn panel_section(title: impl Into<SharedString>, info: Option<(&'static str, &'static str)>, rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let line = style::primary_opacity(cx, 0.06);
    let count = rows.len();
    let mut card = v_flex().rounded(px(8.)).bg(style::primary_opacity(cx, 0.045)).border_1().border_color(style::primary_opacity(cx, 0.06));
    for (i, row) in rows.into_iter().enumerate() {
        card = card.child(div().px(px(10.)).py(px(8.)).text_size(px(style::size::LABEL)).line_height(px(17.)).child(row));
        if i + 1 < count {
            card = card.child(div().ml(px(10.)).h(gpui_kit::px(1.)).bg(line));
        }
    }
    let title: SharedString = title.into();
    let mut heading = h_flex().px(px(4.)).gap(px(6.)).items_center().child(div().text_size(px(style::size::LABEL)).line_height(px(16.)).font_semibold().child(title));
    if let Some((id, text)) = info {
        heading = heading.child(info_tip(id, text, cx));
    }
    v_flex().gap(px(8.)).child(heading).when(count > 0, |s| s.child(card)).into_any_element()
}

/// A label on the left and its control on the right.
pub(crate) fn labeled(label: impl Into<SharedString>, control: AnyElement, _cx: &App) -> AnyElement {
    let label: SharedString = label.into();
    h_flex().w_full().gap(px(10.)).items_center().child(div().flex_1().min_w_0().truncate().child(label)).child(control).into_any_element()
}

/// The same, with the label at the control's top (Position's tall preview).
fn labeled_top(label: &'static str, control: AnyElement, _cx: &App) -> AnyElement {
    h_flex().w_full().gap(px(10.)).items_start().child(div().flex_1().min_w_0().child(label)).child(control).into_any_element()
}

/// A switch with its label on the left and the switch at the right edge.
pub(crate) fn switch_row(id: impl Into<SharedString>, label: impl Into<SharedString>, on: bool, tip: impl Into<SharedString>, disabled: bool, cx: &mut Context<Threadbear>, set: impl Fn(&mut Threadbear, bool, &mut Context<Threadbear>) + 'static) -> AnyElement {
    let label: SharedString = label.into();
    labeled(label, switch(id, on, tip, disabled, cx, set), cx)
}

/// A switch on its own, with its tip, for a row of its own making.
pub(crate) fn switch(id: impl Into<SharedString>, on: bool, tip: impl Into<SharedString>, disabled: bool, cx: &mut Context<Threadbear>, set: impl Fn(&mut Threadbear, bool, &mut Context<Threadbear>) + 'static) -> AnyElement {
    let (id, tip): (SharedString, SharedString) = (id.into(), tip.into());
    let me = cx.entity().downgrade();
    let set = Rc::new(set);
    div()
        .id(SharedString::from(format!("{id}-holder")))
        .test_support()
        .a11y_role(Role::GenericContainer)
        .aria_label(tip.clone())
        .tip(tip)
        .flex_none()
        .child(crate::mac_switch::MacSwitch::new(id, on).disabled(disabled).on_change(move |on, _, cx| {
            let set = set.clone();
            me.update(cx, |this, cx| set(this, on, cx)).ok();
        }))
        .into_any_element()
}

/// A small push button with its tip, for the scripted app to press by it.
pub(crate) fn kit_button(id: impl Into<SharedString>, label: &'static str, symbol: Option<&'static str>, tip: &'static str, disabled: bool, loading: bool, on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> AnyElement {
    let id: SharedString = id.into();
    let mut button = Button::new(SharedString::from(format!("{id}-button"))).label(label).xsmall().secondary().disabled(disabled || loading).loading(loading).tooltip(tip).on_click(on_click);
    if let Some(symbol) = symbol {
        button = button.icon(crate::icons::symbol(symbol));
    }
    div().id(id).test_support().a11y_role(Role::GenericContainer).aria_label(tip).flex_none().child(button).into_any_element()
}

/// A row of choices drawn as one segmented control, out of plain buttons
/// (`SegmentedChoice`), each pressed by its tip.
pub(crate) fn segmented(id: String, options: Vec<(&'static str, &'static str, bool)>, choose: impl Fn(usize, &mut Window, &mut App) + 'static, cx: &App) -> AnyElement {
    let choose = Rc::new(choose);
    let accent = style::control_accent(cx);
    let fg = cx.theme().foreground;
    let mut row = h_flex().w_full().p(px(2.)).gap(px(2.)).rounded(px(8.)).bg(style::primary_opacity(cx, 0.08));
    for (i, (title, tip, on)) in options.into_iter().enumerate() {
        let choose = choose.clone();
        row = row.child(
            crate::highlights::tipped(div().id(SharedString::from(format!("{id}-{i}"))).flex_1().py(px(4.)).rounded(px(6.)).flex().justify_center().cursor_pointer(), tip)
                .when(on, |d| d.bg(accent).text_color(gpui_kit::white()).font_medium())
                .when(!on, |d| d.text_color(fg).hover(|s| s.bg(gpui_kit::hsla(0., 0., 0.5, 0.12))))
                .text_size(px(style::size::LABEL))
                .line_height(px(16.))
                .when(!on, |d| d.role(Role::Button).on_click(move |_, window, cx| choose(i, window, cx)))
                .child(div().truncate().child(title)),
        );
    }
    row.into_any_element()
}

/// A pop-up of a few choices with the current one ticked (Top, Middle and
/// Bottom; Small, Medium and Large).
pub(crate) fn choice_menu(id: String, current: &'static str, tip: &'static str, choices: Vec<(String, bool)>, disabled: bool, choose: impl Fn(usize, &mut App) + 'static) -> AnyElement {
    let choose = Rc::new(choose);
    let button = Button::new(SharedString::from(format!("{id}-button"))).label(current).xsmall().outline().dropdown_caret(true).disabled(disabled).tooltip(tip).dropdown_menu(move |menu: PopupMenu, _, _| {
        let mut menu = menu;
        for (i, (title, checked)) in choices.iter().enumerate() {
            let choose = choose.clone();
            menu = menu.item(PopupMenuItem::new(title.clone()).checked(*checked).on_click(move |_, _, cx| choose(i, cx)));
        }
        menu
    });
    div().id(SharedString::from(id)).test_support().a11y_role(Role::GenericContainer).aria_label(tip).flex_none().child(button).into_any_element()
}

/// One part's row in Layout: its name, then its switch and its place in
/// columns of their own, so they line up whatever the name or place.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_row(id: String, title: &'static str, on: bool, tip: String, place: Place, cx: &mut Context<Threadbear>, show: impl Fn(&mut Threadbear, bool, &mut Context<Threadbear>) + 'static, set_place: impl Fn(&mut Threadbear, Place, &mut Context<Threadbear>) + 'static) -> AnyElement {
    let me = cx.entity().downgrade();
    let toggle = switch(format!("{id}-shown"), on, tip, false, cx, show);
    let menu = choice_menu(format!("{id}-place"), place.title(), "Top, middle or bottom of the page", Place::ALL.iter().map(|p| (p.title().to_owned(), *p == place)).collect(), !on, move |i, cx| {
        let place = Place::ALL[i];
        me.update(cx, |this, cx| set_place(this, place, cx)).ok();
    });
    h_flex()
        .w_full()
        .gap(px(10.))
        .items_center()
        .child(div().flex_1().min_w_0().truncate().child(title))
        .child(toggle)
        .child(h_flex().w(px(92.)).justify_end().child(menu))
        .into_any_element()
}

/// A line under a section's rows saying what went wrong, in red.
pub(crate) fn problem_line(text: &str, cx: &App) -> AnyElement {
    let _ = cx;
    div().text_size(px(style::size::LABEL)).line_height(px(16.)).text_color(gpui_kit::red()).child(text.to_owned()).into_any_element()
}

/// A quiet line of explanation in a section.
pub(crate) fn note(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    let text: SharedString = text.into();
    div().text_size(px(style::size::LABEL)).line_height(px(16.)).text_color(style::secondary_text(cx)).child(text).into_any_element()
}

/// A page's tile or card fill: faint on the plain page, frosted over a
/// photo (GPUI has no blur behind, so a stronger wash of the window's own
/// colour stands in for the Swift app's `.regularMaterial`).
pub(crate) fn tile_fill(on_picture: bool, cx: &App) -> Hsla {
    if on_picture {
        cx.theme().background.opacity(0.78)
    } else {
        style::primary_opacity(cx, 0.04)
    }
}

#[cfg(test)]
mod tests {
    use super::path;

    #[test]
    fn a_name_never_leaves_the_folder() {
        assert!(path("").is_none());
        assert!(path("../x.png").is_none());
        assert!(path("a/b.png").is_none());
        assert!(path("..").is_none());
        assert!(path("picture-1234.png").is_some_and(|p| p.ends_with("looks/picture-1234.png")));
    }
}

impl Threadbear {
    /// The Look parts in the dump: which pages are customizable, each look
    /// as kept, the panel that's open, and the main window's picture.
    #[cfg(feature = "automation")]
    pub(crate) fn looks_dump(&self) -> Value {
        serde_json::json!({
            "on": {
                "librarian": self.customizable(Page::Librarian),
                "scheduled": self.customizable(Page::Scheduled),
                "projects": self.customizable(Page::Projects),
            },
            "editing": self.looks.editing.map(Page::name),
            "librarian": self.customized_librarian_dump(),
            "scheduled": StoredLook::to_value(&self.look::<ScheduledLook>()),
            "projects": StoredLook::to_value(&self.look::<ProjectsLook>()),
            "main": { "on": self.main_background().on, "file": self.main_background().file, "everywhere": self.main_background().everywhere },
            "problems": self.looks.problems,
        })
    }
}

/// The scripted app's `look`: a page made customizable or not (`on`), its
/// panel opened or closed (`customize`), a picture used as if chosen or
/// dropped (`picture`, `background`: a file's path; `page` "main" for the
/// main window's), or the look's fields set (`set`, merged into its JSON).
#[cfg(feature = "automation")]
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, cx: &mut App) -> Result<Value, String> {
    let page_name = cmd["page"].as_str().unwrap_or("librarian");
    let slot = if page_name == "main" { Slot::Main } else { Slot::Page(Page::from_name(page_name).ok_or_else(|| format!("page must be librarian, scheduled, projects or main, not {page_name:?}"))?) };
    app.update(cx, |t, cx| {
        if let (Some(on), Slot::Page(page)) = (cmd["on"].as_bool(), slot) {
            t.set_customizable(page, on, cx);
        }
        if let (Some(open), Slot::Page(page)) = (cmd["customize"].as_bool(), slot) {
            t.looks.editing = open.then_some(page);
            if open && page == Page::Librarian { t.load_profile_page(cx); }
        }
        if let Some(on) = cmd["mainOn"].as_bool() {
            t.edit_main_background(cx, |b| b.on = on);
        }
        if let Some(path) = cmd["picture"].as_str() {
            t.use_look_picture(slot, Well::Picture, PathBuf::from(path), cx);
        }
        if let Some(path) = cmd["background"].as_str() {
            t.use_look_picture(slot, Well::Background, PathBuf::from(path), cx);
        }
        if let (Some(set), Slot::Page(page)) = (cmd["set"].as_object(), slot) {
            let mut v = match page {
                Page::Librarian => StoredLook::to_value(&t.look::<LibrarianLook>()),
                Page::Scheduled => StoredLook::to_value(&t.look::<ScheduledLook>()),
                Page::Projects => StoredLook::to_value(&t.look::<ProjectsLook>()),
            };
            for (k, val) in set {
                v[k] = val.clone();
            }
            match page {
                Page::Librarian => t.save_look(<LibrarianLook as StoredLook>::decode(&v), cx),
                Page::Scheduled => t.save_look(<ScheduledLook as StoredLook>::decode(&v), cx),
                Page::Projects => t.save_look(<ProjectsLook as StoredLook>::decode(&v), cx),
            }
        }
        cx.notify();
    });
    Ok(Value::from("ok"))
}
