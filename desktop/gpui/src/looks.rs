// Pictures of the user's own on Threadbear's pages, a port of the Swift app's
// Looks.swift: the Librarian and Scheduled pages' pictures and backgrounds
// (their Customize panels, looks_editor.rs) and the photo behind a new thread
// (Settings > General > Look). All off until turned on, kept in this app's
// settings under the Swift app's keys, and never sent to the runner. Chosen
// pictures are copied into the data folder's `looks/`, which the Swift app
// uses too, scaled down, so moving or deleting the original doesn't break
// the page; replacing or removing one deletes its copy.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::style;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
// The zoom-aware px (style.rs), in place of GPUI's.
use crate::style::px;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;
use threadbear_core::looks::{self as rules, BackdropFocus, LibrarianLook, Picture, PictureLook, ScheduledLook};

/// The places a background photo is drawn (`BackdropSlot`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Slot {
    Main,
    Librarian,
    Scheduled,
}

/// Which of a page's two pictures a file is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Picture,
    Background,
}

/// The app's side of the looks: which page's Customize panel is open, what
/// went wrong last, and the panels' fields.
pub(crate) struct LooksState {
    pub(crate) editing: Option<Slot>,
    /// "That file isn't a picture", under the section it came from.
    pub(crate) problem: Option<(Slot, Kind, String)>,
    pub(crate) greeting_name: Option<Entity<gpui_kit::component::input::InputState>>,
    pub(crate) github_name: Option<Entity<gpui_kit::component::input::InputState>>,
    /// What the GitHub field says, typed, so a name that isn't one can say so.
    pub(crate) github_typed: String,
    fades: HashMap<Slot, Entity<SliderState>>,
    pub(crate) github: crate::looks_pages::GitHubActivity,
    pub(crate) github_year: Option<i32>,
    pub(crate) activity_year: Option<i32>,
    /// The calendar day under the pointer, by card.
    pub(crate) hovered: Option<(&'static str, threadbear_core::looks::Day)>,
    /// The minute the greeting and the quote were last drawn for, so they
    /// move on by themselves.
    pub(crate) ticking: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl LooksState {
    pub(crate) fn keep(&mut self, sub: Subscription) {
        self._subs.push(sub);
    }

    pub(crate) fn new() -> Self {
        LooksState { editing: None, problem: None, greeting_name: None, github_name: None, github_typed: String::new(), fades: HashMap::new(), github: Default::default(), github_year: None, activity_year: None, hovered: None, ticking: None, _subs: Vec::new() }
    }
}

/// `looks/` in this app's data folder: a dev or mock runner's own, never the
/// real one's, as every runner has its own folder.
pub(crate) fn folder() -> PathBuf {
    threadbear_client::config::RunnerConfig::from_env().home.join("looks")
}

/// A picture in `looks/` that's there, by name. Never a path from elsewhere.
pub(crate) fn path(name: Option<&str>) -> Option<PathBuf> {
    let name = name.filter(|n| rules::safe_name(n))?;
    let path = folder().join(name);
    path.is_file().then_some(path)
}

/// Its size in pixels, read once from the file's header. Names are never
/// reused (each copy gets a new one), so nothing here goes stale.
pub(crate) fn size(name: Option<&str>) -> Option<(f64, f64)> {
    static SIZES: Mutex<Option<HashMap<String, (f64, f64)>>> = Mutex::new(None);
    let name = name?;
    if let Some(size) = SIZES.lock().ok()?.get_or_insert_with(HashMap::new).get(name) {
        return Some(*size);
    }
    let (w, h) = image::image_dimensions(path(Some(name))?).ok()?;
    let size = (f64::from(w), f64::from(h));
    SIZES.lock().ok()?.get_or_insert_with(HashMap::new).insert(name.to_owned(), size);
    Some(size)
}

pub(crate) fn remove(name: &str) {
    if rules::safe_name(name) {
        let _ = std::fs::remove_file(folder().join(name));
    }
}

/// A new name in `looks/`, as the Swift app makes them: `picture-1a2b3c4d.png`.
fn new_name(prefix: &str, ext: &str) -> String {
    use std::hash::{BuildHasher as _, Hasher as _};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    format!("{prefix}-{:08x}.{ext}", h.finish() as u32)
}

/// Copies a picture in, scaled so its longest side is at most `max_side`
/// pixels: one with see-through parts stays a PNG, a photo becomes a JPEG.
/// With `animated` (backgrounds), an animated GIF is copied as it is. Off the
/// main thread: a big photo takes a moment to scale.
pub(crate) fn import(from: &Path, max_side: u32, prefix: &str, animated: bool) -> Result<String, String> {
    const NOT_A_PICTURE: &str = "That file isn't a picture Threadbear can read.";
    let data = std::fs::read(from).map_err(|_| NOT_A_PICTURE.to_owned())?;
    let format = image::guess_format(&data).map_err(|_| NOT_A_PICTURE.to_owned())?;
    let dir = folder();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't save the picture: {e}"))?;
    if animated && format == image::ImageFormat::Gif && gif_frames(&data) > 1 {
        let name = new_name(prefix, "gif");
        std::fs::write(dir.join(&name), &data).map_err(|e| format!("Couldn't save the picture: {e}"))?;
        return Ok(name);
    }
    let picture = image::load_from_memory_with_format(&data, format).map_err(|_| NOT_A_PICTURE.to_owned())?;
    let picture = if picture.width().max(picture.height()) > max_side { picture.resize(max_side, max_side, image::imageops::FilterType::Lanczos3) } else { picture };
    let see_through = picture.color().has_alpha() && picture.to_rgba8().pixels().any(|p| p.0[3] < 255);
    let (ext, bytes) = if see_through {
        let mut out = std::io::Cursor::new(Vec::new());
        picture.write_to(&mut out, image::ImageFormat::Png).map_err(|_| NOT_A_PICTURE.to_owned())?;
        ("png", out.into_inner())
    } else {
        let mut out = Vec::new();
        let rgb = picture.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&rgb).map_err(|_| NOT_A_PICTURE.to_owned())?;
        ("jpg", out)
    };
    let name = new_name(prefix, ext);
    std::fs::write(dir.join(&name), bytes).map_err(|e| format!("Couldn't save the picture: {e}"))?;
    Ok(name)
}

/// How many frames a GIF has, told by what's in it, not its name: a picture
/// dropped from a browser comes without one. Two is enough to know.
fn gif_frames(data: &[u8]) -> usize {
    use image::AnimationDecoder as _;
    image::codecs::gif::GifDecoder::new(std::io::Cursor::new(data)).map_or(0, |d| d.into_frames().take(2).count())
}

thread_local! {
    /// The size each background was last drawn at, for Position's preview.
    static SIZES: RefCell<HashMap<Slot, (f64, f64)>> = RefCell::new(HashMap::new());
    /// A Position drag under way: the slot, the focus it started at, and
    /// where the pointer went down.
    static DRAG: RefCell<Option<(Slot, BackdropFocus, Point<Pixels>)>> = const { RefCell::new(None) };
}

/// The shape to preview: as drawn, or 16:10.
fn drawn_size(slot: Slot) -> (f64, f64) {
    SIZES.with_borrow(|s| s.get(&slot).copied()).or_else(|| SIZES.with_borrow(|s| s.get(&Slot::Main).copied())).unwrap_or((1600., 1000.))
}

/// `BackdropPicture`: a photo filling the space behind a page, cropped round
/// `focus`, with the window's own colour laid over it (`dim`) so the text on
/// top stays readable. Notes the size it's drawn at, for Position. Nothing
/// when the picture's gone.
pub(crate) fn backdrop(slot: Slot, name: Option<&str>, dim: f64, focus: BackdropFocus, cx: &App) -> Option<AnyElement> {
    let file = path(name)?;
    let image = size(name)?;
    let space = drawn_size(slot);
    let (x, y, w, h) = focus.frame(image, space);
    let veil = cx.theme().background.opacity(dim.clamp(0., 1.) as f32);
    // Real pixels, not zoomed: the frame is worked out from the drawn size.
    let p = |v: f64| gpui_kit::px(v as f32);
    Some(
        div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .child(img(file).absolute().left(p(x)).top(p(y)).w(p(w)).h(p(h)))
            .child(div().absolute().inset_0().bg(veil))
            .child(
                canvas(
                    move |bounds, window, _| {
                        let now = (f64::from(f32::from(bounds.size.width)), f64::from(f32::from(bounds.size.height)));
                        if now.0 > 1. && now.1 > 1. && SIZES.with_borrow_mut(|s| s.insert(slot, now) != Some(now)) {
                            // Drawn at the last size; draw again at this one.
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .into_any_element(),
    )
}

/// `BackdropPositioner`: the photo in a frame the shape of where it's drawn.
/// Drag it to choose which part shows; Bottom puts it back on the bottom
/// edge. The page behind follows as you drag. `set` keeps each new focus.
pub(crate) fn positioner(slot: Slot, name: &str, focus: BackdropFocus, max: (f32, f32), set: Rc<dyn Fn(BackdropFocus, &mut App)>, cx: &App) -> Option<AnyElement> {
    let file = path(Some(name))?;
    let image = size(Some(name))?;
    let space = drawn_size(slot);
    let ratio = (space.0 / space.1.max(1.)).clamp(0.4, 3.) as f32;
    // As wide as it can be without growing past the height, so a tall
    // window doesn't make a tower.
    let width = max.0.min(max.1 * ratio);
    let height = width / ratio;
    let preview = (f64::from(width), f64::from(height));
    let (x, y, w, h) = focus.frame(image, preview);
    let can_move = w - preview.0 > 0.5 || h - preview.1 > 0.5;
    let dragging = DRAG.with_borrow(|d| d.is_some_and(|d| d.0 == slot));
    let border = if dragging { style::control_accent(cx) } else { style::primary_opacity(cx, 0.2) };
    let tip = if can_move { "Drag the photo to choose which part shows in the window" } else { "The photo fits the window's shape exactly, so there's nothing to move" };
    let zoom = f64::from(style::zoomed(1.));
    let down_set = set.clone();
    let watch = dragging.then(|| {
        let (moving, up) = (set.clone(), set.clone());
        canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let moving = moving.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    let Some((_, start, at)) = DRAG.with_borrow(|d| *d) else { return };
                    // Points at 100%, as the preview's frame is worked out.
                    let by = (f64::from(f32::from(event.position.x - at.x)) / zoom, f64::from(f32::from(event.position.y - at.y)) / zoom);
                    moving(start.dragged(by, image, preview), cx);
                    window.refresh();
                });
                let _ = &up;
                window.on_mouse_event(move |_: &MouseUpEvent, phase, window, _| {
                    if phase.bubble() {
                        DRAG.with_borrow_mut(|d| *d = None);
                        window.refresh();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    });
    let frame = div()
        .id(SharedString::from(format!("look-position-{slot:?}")))
        .test_support()
        .a11y_role(Role::Image)
        .aria_label(tip)
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(height))
        .overflow_hidden()
        .rounded(px(6.))
        .border(if dragging { gpui_kit::px(2.) } else { gpui_kit::px(1.) })
        .border_color(border)
        .when(can_move, |s| s.cursor(if dragging { CursorStyle::ClosedHand } else { CursorStyle::OpenHand }))
        .tip(tip)
        .child(img(file).absolute().left(px(x as f32)).top(px(y as f32)).w(px(w as f32)).h(px(h as f32)))
        .on_mouse_down(MouseButton::Left, move |event, window, _| {
            if can_move {
                DRAG.with_borrow_mut(|d| *d = Some((slot, focus, event.position)));
                window.refresh();
            }
        })
        .children(watch);
    let secondary = style::secondary_text(cx);
    Some(
        v_flex()
            .items_end()
            .gap(px(6.))
            .child(frame)
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(div().text_size(px(style::size::LABEL)).text_color(secondary).whitespace_nowrap().child(if can_move { "Drag to position" } else { "Fits the window" }))
                    .child(
                        div().id(SharedString::from(format!("look-bottom-{slot:?}"))).test_support().a11y_role(Role::GenericContainer).aria_label("Put the photo back with its bottom edge on the window's bottom edge").child(
                            Button::new(SharedString::from(format!("look-bottom-button-{slot:?}")))
                                .label("Bottom")
                                .xsmall()
                                .secondary()
                                .disabled(focus == BackdropFocus::BOTTOM)
                                .tooltip("Put the photo back with its bottom edge on the window's bottom edge")
                                .on_click(move |_, _, cx| down_set(BackdropFocus::BOTTOM, cx)),
                        ),
                    ),
            )
            .into_any_element(),
    )
}

impl Threadbear {
    pub(crate) fn librarian_customizable(&self) -> bool {
        self.settings.saved.librarian_look_on == Some(true)
    }

    pub(crate) fn scheduled_customizable(&self) -> bool {
        self.settings.saved.scheduled_look_on == Some(true)
    }

    pub(crate) fn librarian_look(&self) -> LibrarianLook {
        LibrarianLook::load(self.settings.saved.librarian_look.as_ref())
    }

    pub(crate) fn scheduled_look(&self) -> ScheduledLook {
        ScheduledLook::load(self.settings.saved.scheduled_look.as_ref())
    }

    /// Keeps a look, and deletes the pictures it no longer uses.
    pub(crate) fn set_librarian_look(&mut self, look: LibrarianLook, cx: &mut Context<Self>) {
        let old = self.librarian_look();
        for file in rules::unused(&old, &look) {
            remove(&file);
        }
        self.settings.saved.librarian_look = Some(look.save());
        self.save_settings_soon(cx);
        cx.notify();
    }

    pub(crate) fn set_scheduled_look(&mut self, look: ScheduledLook, cx: &mut Context<Self>) {
        let old = self.scheduled_look();
        for file in rules::unused(&old, &look) {
            remove(&file);
        }
        self.settings.saved.scheduled_look = Some(look.save());
        self.save_settings_soon(cx);
        cx.notify();
    }

    /// A change to a page's picture or background, whichever page it is.
    pub(crate) fn edit_picture_look(&mut self, slot: Slot, change: impl FnOnce(&mut dyn PictureLook), cx: &mut Context<Self>) {
        match slot {
            Slot::Librarian => {
                let mut look = self.librarian_look();
                change(&mut look);
                self.set_librarian_look(look, cx);
            }
            Slot::Scheduled => {
                let mut look = self.scheduled_look();
                change(&mut look);
                self.set_scheduled_look(look, cx);
            }
            Slot::Main => {}
        }
    }

    pub(crate) fn picture_look(&self, slot: Slot) -> Option<Box<dyn PictureLook>> {
        match slot {
            Slot::Librarian => Some(Box::new(self.librarian_look())),
            Slot::Scheduled => Some(Box::new(self.scheduled_look())),
            Slot::Main => None,
        }
    }

    // Settings > General > Look.

    pub(crate) fn set_look_switch(&mut self, slot: Slot, on: bool, cx: &mut Context<Self>) {
        match slot {
            Slot::Librarian => self.settings.saved.librarian_look_on = Some(on),
            Slot::Scheduled => self.settings.saved.scheduled_look_on = Some(on),
            Slot::Main => self.settings.saved.main_background_on = Some(on),
        }
        if !on && self.looks.editing == Some(slot) {
            self.looks.editing = None;
        }
        self.save_settings_soon(cx);
        cx.notify();
    }

    pub(crate) fn main_background(&self) -> Option<String> {
        self.settings.saved.main_background.clone().filter(|n| rules::safe_name(n))
    }

    pub(crate) fn main_background_dim(&self) -> f64 {
        self.settings.saved.main_background_dim.unwrap_or(rules::DEFAULT_DIM).clamp(0., 1.)
    }

    pub(crate) fn main_background_focus(&self) -> BackdropFocus {
        let s = &self.settings.saved;
        BackdropFocus::new(s.main_background_x.unwrap_or(0.5), s.main_background_y.unwrap_or(1.))
    }

    pub(crate) fn set_main_background_focus(&mut self, focus: BackdropFocus, cx: &mut Context<Self>) {
        self.settings.saved.main_background_x = Some(focus.x);
        self.settings.saved.main_background_y = Some(focus.y);
        self.save_settings_soon(cx);
        cx.notify();
    }

    pub(crate) fn set_main_background_everywhere(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.saved.main_background_everywhere = Some(on);
        self.save_settings_soon(cx);
        cx.notify();
    }

    /// A new background (or none), deleting the old copy. A new photo starts
    /// on the bottom edge, not where the last one was.
    pub(crate) fn set_main_background(&mut self, name: Option<String>, cx: &mut Context<Self>) {
        let old = self.settings.saved.main_background.take();
        if let Some(old) = old.filter(|o| Some(o) != name.as_ref()) {
            remove(&old);
        }
        self.settings.saved.main_background = name;
        self.settings.saved.main_background_x = None;
        self.settings.saved.main_background_y = None;
        self.save_settings_soon(cx);
        cx.notify();
    }

    /// The main window's photo behind a new thread, or behind every thread
    /// with Show behind open threads too, faded a little more there as
    /// there's more text to read (`MainBackdrop`).
    pub(crate) fn render_main_backdrop(&self, is_new: bool, cx: &App) -> Option<AnyElement> {
        let s = &self.settings.saved;
        if s.main_background_on != Some(true) || !(is_new || s.main_background_everywhere == Some(true)) {
            return None;
        }
        let dim = self.main_background_dim();
        backdrop(Slot::Main, self.main_background().as_deref(), if is_new { dim } else { (dim + 0.2).min(0.95) }, self.main_background_focus(), cx)
    }

    // Choosing and dropping pictures.

    /// An open panel for one picture, then a copy of it in `looks/`.
    pub(crate) fn choose_look_picture(&mut self, slot: Slot, kind: Kind, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some(if kind == Kind::Background { "Use as Background".into() } else { "Use Picture".into() }) });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                if let Some(path) = paths.into_iter().next() {
                    this.update(cx, |this, cx| this.import_look_picture(slot, kind, path, cx)).ok();
                }
            }
        })
        .detach();
    }

    /// A picture chosen or dropped on a well: copied in off the main thread,
    /// then used in one change, so the new file and the choice land together.
    pub(crate) fn import_look_picture(&mut self, slot: Slot, kind: Kind, from: PathBuf, cx: &mut Context<Self>) {
        let (max_side, prefix, animated) = match (slot, kind) {
            (Slot::Main, _) => (2880, "window", true),
            (_, Kind::Background) => (2880, "background", true),
            (_, Kind::Picture) => (1200, "picture", false),
        };
        let job = cx.background_spawn(async move { import(&from, max_side, prefix, animated) });
        cx.spawn(async move |this, cx| {
            let got = job.await;
            this.update(cx, |this, cx| {
                let name = match got {
                    Ok(name) => name,
                    Err(e) => {
                        this.looks.problem = Some((slot, kind, e));
                        cx.notify();
                        return;
                    }
                };
                if this.looks.problem.as_ref().is_some_and(|p| p.0 == slot && p.1 == kind) {
                    this.looks.problem = None;
                }
                if slot == Slot::Main {
                    this.set_main_background(Some(name), cx);
                    return;
                }
                this.edit_picture_look(
                    slot,
                    |look| match kind {
                        Kind::Picture => {
                            look.set_picture_file(Some(name));
                            look.set_picture(Picture::Own);
                            look.show_picture();
                        }
                        Kind::Background => {
                            look.set_background(Some(name));
                            look.set_background_focus(BackdropFocus::BOTTOM);
                        }
                    },
                    cx,
                );
            })
            .ok();
        })
        .detach();
    }

    /// The Fade slider for a page's background or the main window's, made
    /// once and kept, so dragging it isn't cut off by a redraw.
    pub(crate) fn fade_slider(&mut self, slot: Slot, now: f64, cx: &mut Context<Self>) -> Entity<SliderState> {
        if let Some(slider) = self.looks.fades.get(&slot) {
            return slider.clone();
        }
        let slider = cx.new(|_| SliderState::new().min(0.).max(rules::MAX_DIM as f32).step(0.01).default_value(now.min(rules::MAX_DIM) as f32));
        let sub = cx.subscribe(&slider, move |this, _, ev: &SliderEvent, cx| {
            let v = match ev {
                SliderEvent::Change(v) | SliderEvent::Release(v) => f64::from(v.start()),
            };
            match slot {
                Slot::Main => {
                    this.settings.saved.main_background_dim = Some(v);
                    this.save_settings_soon(cx);
                    cx.notify();
                }
                _ => this.edit_picture_look(slot, |look| look.set_background_dim(v), cx),
            }
        });
        self.looks._subs.push(sub);
        self.looks.fades.insert(slot, slider.clone());
        slider
    }

    /// Forget a slider whose value changed underneath it (Reset), so the
    /// next one starts where the look is.
    pub(crate) fn forget_fade(&mut self, slot: Slot) {
        self.looks.fades.remove(&slot);
    }
}

/// `PictureWell`: the picture, or a dashed box to drop one on, which takes a
/// file from Finder. `fill` crops a photo to the well; a picture fits inside.
pub(crate) fn well(id: String, name: Option<&str>, empty: &'static str, fill: bool, dropped: Rc<dyn Fn(PathBuf, &mut Window, &mut App)>, cx: &App) -> AnyElement {
    let (accent, faint, secondary) = (style::control_accent(cx), style::primary_opacity(cx, 0.2), style::secondary_text(cx));
    let file = path(name);
    let has = file.is_some();
    div()
        .id(SharedString::from(id))
        .test_support()
        .a11y_role(Role::GenericContainer)
        .aria_label("Drop a picture here from Finder")
        .w_full()
        .h(px(if has { 120. } else { 64. }))
        .rounded(px(8.))
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .when(!has, |s| s.border_1().border_dashed().border_color(faint))
        .tip("Drop a picture here from Finder")
        .drag_over::<ExternalPaths>(move |s, _, _, _| s.border_2().border_color(accent))
        .on_drop(move |paths: &ExternalPaths, window, cx| {
            if let Some(path) = paths.paths().first() {
                dropped(path.clone(), window, cx);
            }
        })
        .map(|s| match file {
            Some(file) => s.child(img(file).size_full().object_fit(if fill { ObjectFit::Cover } else { ObjectFit::Contain })),
            None => s.child(div().text_size(px(style::size::LABEL)).text_color(secondary).child(empty)),
        })
        .into_any_element()
}

impl Threadbear {
    /// A customized page's look, for the dump's `homePage.look`: null while
    /// its Customize switch is off.
    #[cfg(feature = "automation")]
    pub(crate) fn look_dump(&self, slot: Slot) -> serde_json::Value {
        use serde_json::json;
        let (on, layout, picture, background, numbers): (bool, Vec<&str>, Picture, Option<String>, Vec<String>) = match slot {
            Slot::Librarian => {
                let l = self.librarian_look();
                (self.librarian_customizable(), l.layout().into_iter().map(|b| b.raw()).collect(), l.picture, l.background.clone(), l.hidden_numbers.iter().cloned().collect())
            }
            Slot::Scheduled => {
                let l = self.scheduled_look();
                (self.scheduled_customizable(), l.layout().into_iter().map(|b| b.raw()).collect(), l.picture, l.background.clone(), l.hidden_numbers.iter().cloned().collect())
            }
            Slot::Main => return json!({ "on": self.settings.saved.main_background_on == Some(true), "background": self.main_background() }),
        };
        if !on {
            return serde_json::Value::Null;
        }
        json!({ "editing": self.looks.editing == Some(slot), "layout": layout, "picture": picture.raw(), "background": background, "hiddenNumbers": numbers })
    }
}
