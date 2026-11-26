// The plan board's tab beside a thread (experiment planBoard,
// runner/src/experiments/planBoard.md). Its rules are in
// desktop/core/src/plan_board.rs; this draws them and takes the pointer.
//
// It's one thread's plan, never a board of every thread (Mike took
// threadCanvas out for that on 24 Nov). The only ways in are the header's
// Show plan, shown once the thread's agent has drawn a plan, and the
// transcript's "Drew a plan" line. Boxes, dashed groups and arrows with
// handwritten labels sit on a dotted board that pans and zooms as the
// review canvas does. A click puts a box in the thread's own message box as
// a chip; a drag moves it (kept by the runner, by id); Approve plan approves
// the version shown; Send to… starts one nested thread per box. Each sent
// box's badge reads its thread's summary through the inbox's own rules, so
// Needs you here is the same Needs you the Inbox, notifications and the
// Dock badge already show. Nothing here notifies.
use crate::app::Threadbear;
use crate::call;
use crate::style::{self, px, size};
use crate::tips::Tip as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use threadbear_client::runner::Client;
use threadbear_core::plan_board::{self as rules, Board, BoxStatus, Camera, Hit, Rect};

const PANEL_PREFIX: &str = "threadbear.planboard:";
const CANVAS_LIGHT: u32 = 0xF6F6F8;
const CANVAS_DARK: u32 = 0x17171B;
const GREEN: u32 = 0x2E9E5B;
const INK: u32 = 0x6B6B78;

/// The tab's id for a thread's board.
pub(crate) fn panel(thread: &str) -> String {
    format!("{PANEL_PREFIX}{thread}")
}

/// The thread a board tab is for.
pub(crate) fn board_thread(panel: &str) -> Option<&str> {
    panel.strip_prefix(PANEL_PREFIX).filter(|t| !t.is_empty())
}

/// The open boards' views, and which threads are known to have a board.
#[derive(Default)]
pub(crate) struct PlanBoards {
    views: HashMap<String, Entity<PlanBoardView>>,
    // thread → (events seen, has a board), so the header doesn't scan a
    // long transcript every frame.
    has: HashMap<String, (usize, bool)>,
}

impl Global for PlanBoards {}

/// A `board.updated` from the runner: the board's tab, if open, reads the
/// board again (it carries Wayne's own state too, which the event doesn't
/// change). Called from the app's update, so it reads no app state.
pub(crate) fn receive_event(params: &Value, cx: &mut App) {
    if params["event"]["type"].as_str() != Some("board.updated") {
        return;
    }
    let Some(thread) = params["threadId"].as_str() else {
        return;
    };
    if !cx.has_global::<PlanBoards>() {
        return;
    }
    let boards = cx.global_mut::<PlanBoards>();
    if let Some(e) = boards.has.get_mut(thread) {
        e.1 = true;
    } else {
        boards.has.insert(thread.to_owned(), (0, true));
    }
    let view = boards.views.get(thread).cloned();
    if let Some(view) = view {
        let board = Board::decode(&params["event"]["board"]);
        view.update(cx, |v, cx| v.agent_wrote(board, cx));
    }
}

impl Threadbear {
    pub(crate) fn plan_board_on(&self) -> bool {
        self.experiment_on(rules::EXPERIMENT)
    }

    /// Show plan: the board as a tab beside the thread.
    pub(crate) fn show_plan(&mut self, thread: &str, cx: &mut Context<Self>) {
        if !self.plan_board_on() {
            return;
        }
        // Beside the transcript, not over it, so the thread's own message
        // box stays in sight for the chips a click puts there.
        let id = panel(thread);
        self.dock.update(cx, |m, cx| {
            m.show(
                &id,
                &[],
                threadbear_dock::DockDrop::window(threadbear_dock::DockEdge::Right),
                cx,
            )
        });
    }

    /// The header's Show plan, for a thread whose agent drew a plan.
    pub(crate) fn render_show_plan_button(
        &mut self,
        thread: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.plan_board_on() {
            return None;
        }
        let pane = self.pane.clone()?;
        if pane.read(cx).summary.id != thread {
            return None;
        }
        let has = {
            let (id, n) = {
                let p = pane.read(cx);
                (p.summary.id.clone(), p.feed.model().events().len())
            };
            let cached = cx.default_global::<PlanBoards>().has.get(&id).copied();
            match cached {
                Some((seen, has)) if seen == n || has => has,
                _ => {
                    let has = pane
                        .read(cx)
                        .feed
                        .model()
                        .events()
                        .iter()
                        .any(|e| e.type_ == "board.updated");
                    cx.default_global::<PlanBoards>().has.insert(id, (n, has));
                    has
                }
            }
        };
        if !has {
            return None;
        }
        let id = thread.to_owned();
        let secondary = style::secondary_text(cx);
        Some(
            crate::a11y::A11yRole::a11y_role(
                div()
                    .id("header-show-plan")
                    .h(px(26.))
                    .px(px(8.))
                    .mr(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .rounded(px(6.))
                    .hover(|s| s.bg(style::hover_fill(cx)))
                    .text_size(px(size::LABEL))
                    .text_color(secondary)
                    .test_support(),
                Role::Button,
            )
            .aria_label("Show plan")
            .tip("Show plan")
            .child(crate::highlights::icon(
                "point.3.connected.trianglepath.dotted",
                size::LABEL,
                secondary,
            ))
            .child("Show plan")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_plan(&id, cx)))
            .into_any_element(),
        )
    }

    pub(crate) fn plan_board_title(&self, panel: &str) -> Option<String> {
        let thread = board_thread(panel)?;
        // Its thread's name too: the tab can sit beside another thread.
        Some(match self.threads.get(thread) {
            Some(t) if !t.title.is_empty() => format!("Plan · {}", t.title),
            _ => "Plan".to_owned(),
        })
    }

    /// The board tab's view, one per thread, kept while the tab is open.
    pub(crate) fn plan_board_view(
        &mut self,
        panel: &str,
        ghost: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(thread) = board_thread(panel).map(str::to_owned) else {
            return div().into_any_element();
        };
        if ghost {
            return div().size_full().into_any_element();
        }
        if !self.plan_board_on() {
            return crate::files::pane_note(
                "Plan board is off. Turn it on in Settings > General > Experiments.",
                crate::highlights::palette(cx),
            )
            .into_any_element();
        }
        let app = cx.entity().downgrade();
        let client = self.client.clone();
        let existing = cx
            .default_global::<PlanBoards>()
            .views
            .get(&thread)
            .cloned();
        let view = match existing {
            Some(v) => v,
            None => {
                let dock = self.dock.clone();
                let v = cx.new(|cx| PlanBoardView::new(app, client, thread.clone(), dock, cx));
                cx.default_global::<PlanBoards>()
                    .views
                    .insert(thread, v.clone());
                v
            }
        };
        AnyView::from(view).into_any_element()
    }

    /// Lets go of boards whose tabs have closed.
    pub(crate) fn plan_boards_layout_changed(
        &mut self,
        layout: &threadbear_dock::DockLayout,
        cx: &mut Context<Self>,
    ) {
        if cx.has_global::<PlanBoards>() {
            cx.global_mut::<PlanBoards>()
                .views
                .retain(|id, _| layout.contains(&panel(id)));
        }
    }
}

enum Drag {
    Pan {
        from: (f32, f32),
        camera: Camera,
        moved: bool,
    },
    Box {
        id: String,
        from: (f32, f32),
        start: (f32, f32),
        moved: bool,
        shift: bool,
    },
}

pub(crate) struct PlanBoardView {
    app: WeakEntity<Threadbear>,
    // The window's dock, for Close and Pop out / Put back.
    dock: Entity<threadbear_dock::DockModel>,
    _observe_dock: Subscription,
    client: Arc<Client>,
    thread_id: String,
    board: Option<Board>,
    // Give to the project coordinator's target, when the runner offers it.
    coordinator: Option<(String, String)>,
    loading: bool,
    error: Option<String>,
    camera: Camera,
    fitted: bool,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    drag: Option<Drag>,
    // Positions dragged here and not yet back from the runner.
    moving: BTreeMap<String, (f32, f32)>,
    selection: Vec<Hit>,
    // The chips this board put in the message box, for Esc.
    chips: Vec<String>,
    focus: FocusHandle,
    // Tab's place among the boxes.
    keyed: Option<usize>,
    approving: bool,
    // The skills the board's folder can run, once asked.
    folder_skills: Option<Vec<String>>,
    // The message Ask planner to review sent, so it isn't sent twice.
    asked: Option<String>,
    // Changed badges Wayne has seen, by version.
    seen_changes: Option<u32>,
    // planBoardMockups: the box whose screens are showing, its screens, and
    // their stills by (screen, version, dark).
    screens_for: Option<String>,
    frames: Vec<rules::Frame>,
    stills: HashMap<(String, u32, bool), Option<Arc<Image>>>,
    // One still at a time: they share one sealed view.
    drawing: bool,
    // Watching the web runtime start, once.
    runtime_watch: Option<Subscription>,
    _observe: Option<Subscription>,
}

impl PlanBoardView {
    fn new(
        app: WeakEntity<Threadbear>,
        client: Arc<Client>,
        thread_id: String,
        dock: Entity<threadbear_dock::DockModel>,
        cx: &mut Context<Self>,
    ) -> Self {
        // Pop out and Put back follow where the tab is.
        let observe_dock = cx.observe(&dock, |_, _, cx| cx.notify());
        // Badges follow the threads' summaries: redraw when the app's list changes.
        let observe = app
            .upgrade()
            .map(|a| cx.observe(&a, |_, _, cx| cx.notify()));
        let mut v = PlanBoardView {
            app,
            dock,
            _observe_dock: observe_dock,
            client,
            thread_id,
            board: None,
            coordinator: None,
            loading: true,
            error: None,
            camera: Camera {
                s: 1.,
                tx: 40.,
                ty: 40.,
            },
            fitted: false,
            bounds: Rc::new(Cell::new(None)),
            drag: None,
            moving: BTreeMap::new(),
            selection: Vec::new(),
            chips: Vec::new(),
            focus: cx.focus_handle(),
            keyed: None,
            approving: false,
            folder_skills: None,
            asked: None,
            seen_changes: None,
            screens_for: None,
            frames: Vec::new(),
            stills: HashMap::new(),
            drawing: false,
            runtime_watch: None,
            _observe: observe,
        };
        v.reload(cx);
        v
    }

    /// board.get: the board as the runner has it now.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let params = json!({ "threadId": self.thread_id });
        cx.spawn(async move |this, cx| {
            let res = call(client, "board.get", params).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(v) => {
                        this.error = None;
                        this.take_board(Board::decode(&v["board"]));
                        this.coordinator = v["coordinator"]["projectId"]
                            .as_str()
                            .zip(v["coordinator"]["threadId"].as_str())
                            .map(|(p, t)| (p.to_owned(), t.to_owned()));
                    }
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn take_board(&mut self, board: Option<Board>) {
        if let Some(b) = &board {
            // A move the runner has now confirmed needn't be held here.
            self.moving.retain(|id, at| b.positions.get(id) != Some(at));
        }
        self.board = board;
    }

    /// The agent wrote a new version: draw it at once, then read the rest
    /// (approval, moves, links) from the runner.
    fn agent_wrote(&mut self, board: Option<Board>, cx: &mut Context<Self>) {
        if let Some(b) = board {
            self.take_board(Some(b));
        }
        self.asked = None;
        self.reload(cx);
        cx.notify();
    }

    fn statuses(&self, cx: &App) -> BTreeMap<String, BoxStatus> {
        let Some(board) = &self.board else {
            return BTreeMap::new();
        };
        let app = self.app.upgrade();
        let threads = app.as_ref().map(|a| &a.read(cx).threads);
        rules::statuses(
            board,
            |id| threads.and_then(|t| t.get(id)).map(|t| (**t).clone()),
            // Coordinator to-dos come from Mike's coordinators, which this
            // copy may not have: Can't see it until they're read.
            |_| None,
        )
    }

    fn positions(&self) -> BTreeMap<String, (f32, f32)> {
        let mut p = self
            .board
            .as_ref()
            .map(|b| b.positions.clone())
            .unwrap_or_default();
        // Drags under way and not yet back from the runner.
        p.extend(self.moving.clone());
        p
    }
}

// ---------------------------------------------------------------- actions

impl PlanBoardView {
    /// A board RPC whose answer is the board.
    fn board_call(&mut self, method: &'static str, params: Value, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, method, params).await;
            this.update(cx, |this, cx| {
                this.approving = false;
                match res {
                    Ok(v) => {
                        this.error = None;
                        if let Some(b) = Board::decode(&v["board"]) {
                            this.take_board(Some(b));
                        }
                    }
                    Err(e) => {
                        // Rolled back: the runner's copy is the truth.
                        this.error = Some(e.message);
                        this.moving.clear();
                        this.reload(cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn approve(&mut self, cx: &mut Context<Self>) {
        let Some(b) = &self.board else { return };
        if self.approving || b.state().approved() {
            return;
        }
        self.approving = true;
        // Shown at once; the runner refuses a version that isn't the latest.
        let version = b.version;
        if let Some(b) = self.board.as_mut() {
            b.approved = Some(rules::Approved {
                version,
                at: String::new(),
            });
        }
        self.board_call(
            "board.approve",
            json!({ "threadId": self.thread_id, "version": version }),
            cx,
        );
        cx.notify();
    }

    fn tidy(&mut self, cx: &mut Context<Self>) {
        self.moving.clear();
        if let Some(b) = self.board.as_mut() {
            b.positions.clear();
        }
        self.fitted = false;
        self.board_call(
            "board.move",
            json!({ "threadId": self.thread_id, "positions": {}, "tidy": true }),
            cx,
        );
        cx.notify();
    }

    fn moved(&mut self, id: &str, at: (f32, f32), cx: &mut Context<Self>) {
        self.moving.insert(id.to_owned(), at);
        self.board_call(
            "board.move",
            json!({ "threadId": self.thread_id, "positions": { id: [at.0, at.1] } }),
            cx,
        );
    }

    fn set_skills(&mut self, skills: Vec<String>, cx: &mut Context<Self>) {
        if let Some(b) = self.board.as_mut() {
            b.skills = skills.clone();
        }
        self.board_call(
            "board.skills",
            json!({ "threadId": self.thread_id, "skills": skills }),
            cx,
        );
        cx.notify();
    }

    /// The skills the board's folder can run: the `/` palette's list.
    fn load_folder_skills(&mut self, cx: &mut Context<Self>) {
        if self.folder_skills.is_some() {
            return;
        }
        self.folder_skills = Some(Vec::new());
        let app = self.app.upgrade();
        let summary = app
            .as_ref()
            .and_then(|a| a.read(cx).threads.get(&self.thread_id).cloned());
        let Some(t) = summary else { return };
        let client = self.client.clone();
        let params = json!({ "cwd": t.cwd, "accountId": t.account_id });
        cx.spawn(async move |this, cx| {
            let res = call(client, "folders.commands", params).await;
            this.update(cx, |this, cx| {
                this.folder_skills = Some(skill_names(res.ok().as_ref()));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A box, group or arrow into the thread's own message box as a chip.
    fn select(&mut self, hit: Hit, add: bool, cx: &mut Context<Self>) {
        let Some(board) = &self.board else { return };
        if !add {
            self.selection.clear();
        }
        if !self.selection.contains(&hit) {
            self.selection.push(hit.clone());
        }
        let reference = rules::chip_markdown(board, &hit);
        if !self.chips.contains(&reference) {
            self.chips.push(reference.clone());
        }
        let thread = self.thread_id.clone();
        if let Some(app) = self.app.upgrade() {
            let composer = app.read(cx).composers.get(&thread).cloned();
            if let Some(c) = composer {
                c.update(cx, |c, cx| c.mention_reference(reference, cx));
            }
        }
        cx.notify();
    }

    /// Esc: nothing selected, and this board's chips out of the message box.
    fn clear(&mut self, cx: &mut Context<Self>) -> bool {
        if self.selection.is_empty() && self.chips.is_empty() {
            return false;
        }
        self.selection.clear();
        let chips = std::mem::take(&mut self.chips);
        if let Some(app) = self.app.upgrade() {
            let composer = app.read(cx).composers.get(&self.thread_id).cloned();
            if let Some(c) = composer {
                c.update(cx, |c, cx| c.forget_references(&chips, cx));
            }
        }
        cx.notify();
        true
    }

    /// Ask planner to review: one ordinary message, sent only on this click.
    fn ask_review(&mut self, message: String, cx: &mut Context<Self>) {
        if self.asked.is_some() {
            return;
        }
        self.asked = Some(message.clone());
        let client = self.client.clone();
        let params = json!({ "threadId": self.thread_id, "text": message });
        cx.spawn(async move |this, cx| {
            if let Err(e) = call(client, "threads.send", params).await {
                this.update(cx, |this, cx| {
                    this.asked = None;
                    this.error = Some(e.message);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
    }

    fn open_thread(&self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |a, cx| a.open_thread_link(&id, window, cx));
        }
    }

    fn zoom(&mut self, factor: f32) {
        let (w, h) = self.view_size();
        self.camera = self.camera.zoom_at((w / 2., h / 2.), factor);
    }

    fn view_size(&self) -> (f32, f32) {
        self.bounds
            .get()
            .map(|b| (f32::from(b.size.width), f32::from(b.size.height)))
            .unwrap_or((800., 600.))
    }

    fn fit(&mut self) {
        let Some(b) = &self.board else { return };
        let layout = rules::layout(&b.graph, &self.positions());
        let (w, h) = self.view_size();
        self.camera = Camera::fit(layout.bounds(), w, h, 60., 1.4);
        self.fitted = true;
    }
}

/// The skill names in a folders.commands answer.
fn skill_names(v: Option<&Value>) -> Vec<String> {
    v.and_then(|v| v["commands"].as_array())
        .into_iter()
        .flatten()
        .filter(|c| c["source"].as_str() == Some("skill"))
        .filter_map(|c| {
            c["name"]
                .as_str()
                .map(|n| n.trim_start_matches('/').to_owned())
        })
        .collect()
}

// ---------------------------------------------------------------- input

impl PlanBoardView {
    fn local(&self, at: Point<Pixels>) -> Option<(f32, f32)> {
        let b = self.bounds.get()?;
        Some((f32::from(at.x - b.origin.x), f32::from(at.y - b.origin.y)))
    }

    fn pressed(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.local(ev.position) else {
            return;
        };
        window.focus(&self.focus, cx);
        // Tab, Esc and Return reach the board, not a web view that had them.
        crate::web_panels::keyboard_to_app(window);
        let Some(board) = &self.board else { return };
        let layout = rules::layout(&board.graph, &self.positions());
        let world = self.camera.to_world(at);
        self.drag = Some(match rules::hit(&layout, world, self.camera.s) {
            Some(Hit::Box(id)) => {
                let r = layout.rect(&id).unwrap_or(Rect::new(0., 0., 0., 0.));
                Drag::Box {
                    id,
                    from: at,
                    start: (r.x, r.y),
                    moved: false,
                    shift: ev.modifiers.shift,
                }
            }
            _ => Drag::Pan {
                from: at,
                camera: self.camera,
                moved: false,
            },
        });
    }

    fn mouse_moved(&mut self, ev: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(at) = self.local(ev.position) else {
            return;
        };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            return;
        }
        let s = self.camera.s;
        match self.drag.as_mut() {
            Some(Drag::Pan {
                from,
                camera,
                moved,
            }) => {
                if (at.0 - from.0).abs() + (at.1 - from.1).abs() > 3. {
                    *moved = true;
                }
                self.camera = camera.pan(at.0 - from.0, at.1 - from.1);
            }
            Some(Drag::Box {
                id,
                from,
                start,
                moved,
                ..
            }) => {
                if (at.0 - from.0).abs() + (at.1 - from.1).abs() > 3. {
                    *moved = true;
                }
                if *moved {
                    let p = (start.0 + (at.0 - from.0) / s, start.1 + (at.1 - from.1) / s);
                    self.moving.insert(id.clone(), p);
                }
            }
            None => return,
        }
        cx.notify();
    }

    fn released(&mut self, ev: &MouseUpEvent, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        match drag {
            Drag::Box {
                id, moved: true, ..
            } => {
                if let Some(at) = self.moving.get(&id).copied() {
                    self.moved(&id, at, cx);
                }
            }
            Drag::Box { id, shift, .. } => self.select(Hit::Box(id), shift, cx),
            Drag::Pan { moved: false, .. } => {
                let Some(at) = self.local(ev.position) else {
                    return;
                };
                let Some(board) = &self.board else { return };
                let layout = rules::layout(&board.graph, &self.positions());
                match rules::hit(&layout, self.camera.to_world(at), self.camera.s) {
                    Some(h) => self.select(h, ev.modifiers.shift, cx),
                    None => {
                        self.selection.clear();
                        cx.notify();
                    }
                }
            }
            Drag::Pan { .. } => {}
        }
    }

    fn scrolled(&mut self, ev: &ScrollEvent, cx: &mut Context<Self>) {
        let Some(at) = self.local(ev.position) else {
            return;
        };
        let cam = self.camera;
        let wheel = ev.source == ScrollSource::Wheel;
        // As the review canvas: a wheel zooms round the pointer, two fingers
        // move, ⌘ or Control with them zooms.
        self.camera = match ev.delta {
            ScrollDelta::Lines(l) => cam.zoom_at(at, rules::wheel_zoom(l.y)),
            ScrollDelta::Pixels(p) if wheel => {
                cam.zoom_at(at, rules::smooth_wheel_zoom(f32::from(p.y)))
            }
            ScrollDelta::Pixels(p) if ev.modifiers.platform || ev.modifiers.control => {
                cam.zoom_at(at, (f32::from(p.y) * 0.01).exp())
            }
            ScrollDelta::Pixels(p) => cam.pan(f32::from(p.x), f32::from(p.y)),
        };
        cx.stop_propagation();
        cx.notify();
    }

    fn pinched(&mut self, ev: &PinchEvent, cx: &mut Context<Self>) {
        let Some(at) = self.local(ev.position) else {
            return;
        };
        self.camera = self.camera.zoom_at(at, 1. + ev.delta);
        cx.notify();
    }

    fn key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(board) = &self.board else {
            return false;
        };
        let order = rules::tab_order(board);
        match ev.keystroke.key.as_str() {
            "escape" => self.clear(cx),
            "tab" if !order.is_empty() => {
                let n = order.len();
                let next = match (self.keyed, ev.keystroke.modifiers.shift) {
                    (None, false) => 0,
                    (None, true) => n - 1,
                    (Some(i), false) => (i + 1) % n,
                    (Some(i), true) => (i + n - 1) % n,
                };
                self.keyed = Some(next);
                cx.notify();
                true
            }
            "enter" => {
                let Some(id) = self.keyed.and_then(|i| order.get(i)).cloned() else {
                    return false;
                };
                if ev.keystroke.modifiers.platform {
                    self.open_send(&id, window, cx);
                } else {
                    self.select(Hit::Box(id), ev.keystroke.modifiers.shift, cx);
                }
                true
            }
            "1" if ev.keystroke.modifiers.shift => {
                self.fit();
                cx.notify();
                true
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------- the header

/// A small text button with its tip, which is also its label to VoiceOver
/// and to scripts.
fn bar_button(
    id: &'static str,
    label: impl Into<SharedString>,
    tip: impl Into<SharedString>,
    enabled: bool,
    cx: &App,
) -> impl StatefulInteractiveElement + ParentElement + IntoElement {
    let label: SharedString = label.into();
    let tip: SharedString = tip.into();
    crate::a11y::A11yRole::a11y_role(
        div()
            .id(id)
            .h(px(24.))
            .px(px(8.))
            .flex()
            .items_center()
            .rounded(px(6.))
            .border_1()
            .border_color(style::button_edge(cx))
            .bg(style::button_fill(cx))
            .text_size(px(size::DETAIL))
            .text_color(style::label_text(cx))
            .when(!enabled, |d| d.opacity(0.4))
            .when(enabled, |d| d.hover(|s| s.bg(style::hover_fill(cx))))
            .test_support(),
        Role::Button,
    )
    .aria_label(tip.clone())
    .tip(tip)
    .child(label)
}

impl PlanBoardView {
    fn render_bar(
        &mut self,
        statuses: &BTreeMap<String, BoxStatus>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(board) = self.board.clone() else {
            return div().into_any_element();
        };
        let state = board.state();
        let words = state.words();
        let state_colour = match state {
            rules::PlanState::Approved(_) => rgb(GREEN).into(),
            rules::PlanState::Changed { .. } => style::needs_you(cx),
            rules::PlanState::Draft(_) => style::secondary_text(cx),
        };
        let mut bar = h_flex()
            .id("plan-board-bar-tools")
            .flex_1()
            .min_w(px(0.))
            .flex_wrap()
            .gap(px(6.))
            .items_center()
            .child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id("plan-board-state")
                        .text_size(px(size::LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(state_colour)
                        .test_support(),
                    Role::Label,
                )
                .aria_label(SharedString::from(words.clone()))
                .child(words),
            );
        if !state.approved() {
            bar = bar.child(
                bar_button(
                    "plan-board-approve",
                    "Approve plan",
                    "Approve plan",
                    !self.approving,
                    cx,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.approve(cx))),
            );
        }
        bar = bar.child(self.skills_menu(&board, cx));
        bar = bar.child(self.versions_menu(&board));
        if let Some(message) = rules::review_message(statuses, &board) {
            let asked = self.asked.is_some();
            let tip = if message.contains("failed") {
                "Ask planner to review (some boxes failed)"
            } else {
                "Ask planner to review"
            };
            bar = bar.child(
                bar_button(
                    "plan-board-review",
                    if asked {
                        "Asked planner"
                    } else {
                        "Ask planner to review"
                    },
                    tip,
                    !asked,
                    cx,
                )
                .when(!asked, |b| {
                    b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.ask_review(message.clone(), cx)
                    }))
                }),
            );
        }
        if board.graph.skipped > 0 {
            let n = board.graph.skipped;
            bar = bar.child(
                div()
                    .text_size(px(size::SMALL))
                    .text_color(style::quiet_text(cx))
                    .child(format!(
                        "{n} line{} not drawn",
                        if n == 1 { "" } else { "s" }
                    )),
            );
        }
        if let Some(e) = self.error.clone() {
            bar = bar.child(
                div()
                    .text_size(px(size::SMALL))
                    .text_color(style::failed(cx))
                    .truncate()
                    .child(e),
            );
        }
        let pct = format!("{}%", (self.camera.s * 100.).round());
        let bar = bar
            .child(div().flex_1())
            .child(
                bar_button("plan-board-zoom-out", "−", "Zoom out", true, cx).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.zoom(1. / 1.25);
                        cx.notify();
                    },
                )),
            )
            .child(
                div()
                    .text_size(px(size::SMALL))
                    .text_color(style::secondary_text(cx))
                    .w(px(40.))
                    .text_center()
                    .child(pct),
            )
            .child(
                bar_button("plan-board-zoom-in", "+", "Zoom in", true, cx).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.zoom(1.25);
                        cx.notify();
                    },
                )),
            )
            .child(
                bar_button("plan-board-fit", "Fit", "Fit the plan (⇧1)", true, cx).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.fit();
                        cx.notify();
                    }),
                ),
            )
            .child(
                bar_button(
                    "plan-board-tidy",
                    "Tidy",
                    "Tidy: lay the plan out again",
                    true,
                    cx,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.tidy(cx))),
            );
        // The tools wrap onto another row when the pane is narrow, so every
        // button stays in reach; Pop out and Close stay at the end.
        // The tab's own × shows only on a hover of its title, which is easy
        // to miss (Wayne, 26 Nov): this one's always there.
        h_flex()
            .id("plan-board-bar")
            .flex_none()
            .w_full()
            .min_h(px(38.))
            .py(px(4.))
            .px(px(10.))
            .gap(px(6.))
            .items_start()
            .border_b_1()
            .border_color(style::hairline(cx))
            .child(bar)
            .child({
                let (id, label, tip) = if self.floating(cx) {
                    (
                        "plan-board-put-back",
                        "↙",
                        "Put the board back beside the thread",
                    )
                } else {
                    (
                        "plan-board-pop-out",
                        "⧉",
                        "Open the board in its own window",
                    )
                };
                div()
                    .flex_none()
                    .child(bar_button(id, label, tip, true, cx).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.pop_out_or_back(window, cx),
                    )))
            })
            .child(div().flex_none().child(
                bar_button("plan-board-close", "✕", "Close the board", true, cx).on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)),
                ),
            ))
            .into_any_element()
    }

    /// Closes the board's tab. After this click is handled: closing it lets
    /// go of this view.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (dock, id) = (self.dock.clone(), panel(&self.thread_id));
        window.defer(cx, move |_, cx| dock.update(cx, |m, cx| m.close(&id, cx)));
    }

    /// Whether the board is in a window of its own.
    fn floating(&self, cx: &App) -> bool {
        self.dock
            .read(cx)
            .layout()
            .is_floating(&panel(&self.thread_id))
    }

    /// Pop out: the board in a window of its own. Put back: beside the
    /// thread again. The tab's right-click menu has both too.
    fn pop_out_or_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (dock, id, out) = (
            self.dock.clone(),
            panel(&self.thread_id),
            !self.floating(cx),
        );
        window.defer(cx, move |_, cx| {
            dock.update(cx, |m, cx| {
                if out {
                    m.open_in_window(&id, cx);
                } else {
                    m.put_back(&id, cx);
                }
            })
        });
    }

    fn skills_menu(&mut self, board: &Board, cx: &mut Context<Self>) -> AnyElement {
        self.load_folder_skills(cx);
        let picked = board.skills.clone();
        let mut available = self.folder_skills.clone().unwrap_or_default();
        for s in &picked {
            if !available.contains(s) {
                available.push(s.clone());
            }
        }
        let label = if picked.is_empty() {
            "Skills".to_owned()
        } else {
            format!("Skills: {}", picked.len())
        };
        let me = cx.entity().downgrade();
        Button::new("plan-board-skills")
            .label(label)
            .xsmall()
            .outline()
            .dropdown_caret(true)
            .tooltip("The skills every box's brief names")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                if available.is_empty() {
                    menu = menu.item(PopupMenuItem::new("No skills in this folder").disabled(true));
                }
                for name in &available {
                    let on = picked.contains(name);
                    let (name, picked, me) = (name.clone(), picked.clone(), me.clone());
                    menu = menu.item(PopupMenuItem::new(format!("/{name}")).checked(on).on_click(
                        move |_, _, cx| {
                            let mut next = picked.clone();
                            if on {
                                next.retain(|s| *s != name);
                            } else {
                                next.push(name.clone());
                            }
                            me.update(cx, |v, cx| v.set_skills(next, cx)).ok();
                        },
                    ));
                }
                menu
            })
            .into_any_element()
    }

    /// The last 10 versions and their notes. Using an older one is asked of
    /// the agent in a message, so it always knows what's on the board.
    fn versions_menu(&mut self, board: &Board) -> AnyElement {
        let versions = board.versions.clone();
        let latest = board.version;
        let (client, thread) = (self.client.clone(), self.thread_id.clone());
        Button::new("plan-board-versions")
            .label(format!("v{latest}"))
            .xsmall()
            .outline()
            .dropdown_caret(true)
            .tooltip("Versions of the plan")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for v in versions.iter().rev() {
                    let note = v.note.clone().unwrap_or_default();
                    let title = if note.is_empty() { format!("v{}", v.version) } else { format!("v{} · {}", v.version, note.chars().take(60).collect::<String>()) };
                    let (client, thread, n) = (client.clone(), thread.clone(), v.version);
                    menu = menu.item(PopupMenuItem::new(title).checked(n == latest).disabled(n == latest).on_click(move |_, _, _| {
                        let (client, thread) = (client.clone(), thread.clone());
                        crate::RT.spawn(async move {
                            let text = format!("Please make v{n} of the plan board the current plan again (read_board, then update_board).");
                            let _ = call(client, "threads.send", json!({ "threadId": thread, "text": text })).await;
                        });
                    }));
                }
                menu
            })
            .into_any_element()
    }
}

// ---------------------------------------------------------------- the canvas

type Pen = (rules::Curve, f32, Hsla, bool);

fn paint_arrows(pens: &[Pen], origin: Point<Pixels>, window: &mut Window) {
    let at = |p: (f32, f32)| point(origin.x + gpui_kit::px(p.0), origin.y + gpui_kit::px(p.1));
    for (c, width, colour, head) in pens {
        let width = width.max(0.6);
        let mut path = PathBuilder::stroke(gpui_kit::px(width));
        path.move_to(at(c.start));
        path.cubic_bezier_to(at(c.end), at(c.c1), at(c.c2));
        if let Ok(path) = path.build() {
            window.paint_path(path, *colour);
        }
        if !head {
            continue;
        }
        let (dx, dy) = (c.end.0 - c.c2.0, c.end.1 - c.c2.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / len, dy / len);
        let h = width * 4.5;
        for side in [-1., 1.] {
            let back = (
                c.end.0 - ux * h - uy * h * 0.6 * side,
                c.end.1 - uy * h + ux * h * 0.6 * side,
            );
            let mut p = PathBuilder::stroke(gpui_kit::px(width));
            p.move_to(at(back));
            p.line_to(at(c.end));
            if let Ok(p) = p.build() {
                window.paint_path(p, *colour);
            }
        }
    }
}

fn paint_dots(cam: Camera, colour: Hsla, bounds: Bounds<Pixels>, window: &mut Window) {
    let mut step = 48.;
    while step * cam.s < 18. {
        step *= 2.;
    }
    while step * cam.s > 36. {
        step /= 2.;
    }
    let gap = step * cam.s;
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let mut y = cam.ty.rem_euclid(gap);
    while y < h {
        let mut x = cam.tx.rem_euclid(gap);
        while x < w {
            window.paint_quad(fill(
                Bounds {
                    origin: point(
                        bounds.origin.x + gpui_kit::px(x - 1.),
                        bounds.origin.y + gpui_kit::px(y - 1.),
                    ),
                    size: Size {
                        width: gpui_kit::px(2.),
                        height: gpui_kit::px(2.),
                    },
                },
                colour,
            ));
            x += gap;
        }
        y += gap;
    }
}

/// The agent's note, a bubble near what it's about.
fn note_bubble(board: &Board, layout: &rules::Layout, cam: Camera, cx: &App) -> Option<AnyElement> {
    let note = board.note.clone().filter(|n| !n.text.is_empty())?;
    let s = cam.s;
    let (edge, label) = (style::hairline(cx), style::label_text(cx));
    let near = note
        .near
        .as_deref()
        .and_then(|n| layout.rect(n))
        .unwrap_or_else(|| layout.bounds());
    let sr = cam.rect(&near);
    let w = 260. * s.clamp(0.5, 1.4);
    Some(
        crate::a11y::A11yRole::a11y_role(
            div()
                .id("plan-note")
                .absolute()
                .left(gpui_kit::px(sr.x))
                .top(gpui_kit::px(sr.y - 70. * s.clamp(0.5, 1.4)))
                .w(gpui_kit::px(w))
                .p(gpui_kit::px(8.))
                .rounded(gpui_kit::px(10.))
                .bg(style::bubble_background(cx))
                .border_1()
                .border_color(edge)
                .text_size(gpui_kit::px(12. * s.clamp(0.7, 1.2)))
                .text_color(label)
                .test_support(),
            Role::Note,
        )
        .aria_label(SharedString::from(format!("Note: {}", note.text)))
        .child(note.text)
        .into_any_element(),
    )
}

fn status_colour(s: &BoxStatus, cx: &App) -> Hsla {
    match s {
        BoxStatus::Working(_) => style::accent(cx),
        BoxStatus::NeedsYou => style::needs_you(cx),
        BoxStatus::Done => rgb(GREEN).into(),
        BoxStatus::Failed => style::failed(cx),
        BoxStatus::Archived | BoxStatus::CantSee => style::quiet_text(cx),
    }
}

impl PlanBoardView {
    #[expect(
        clippy::too_many_lines,
        reason = "one board drawn in layers, read top to bottom"
    )]
    fn render_canvas(
        &mut self,
        statuses: &BTreeMap<String, BoxStatus>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(board) = self.board.clone() else {
            return div().into_any_element();
        };
        let layout = rules::layout(&board.graph, &self.positions());
        if !self.fitted && self.bounds.get().is_some() {
            self.fit();
        }
        let cam = self.camera;
        let s = cam.s;
        let dark = cx.theme().is_dark();
        let label = style::label_text(cx);
        let secondary = style::secondary_text(cx);
        let edge = style::hairline(cx);
        let accent = style::accent(cx);
        let card = style::card_background(cx);
        let ink: Hsla = rgb(INK).into();
        let mut layer = div().absolute().inset_0();

        // Groups: dashed frames with their name at the top.
        for (id, r) in &layout.groups {
            let sr = cam.rect(r);
            let name = board
                .graph
                .group(id)
                .map(|g| g.label.clone())
                .unwrap_or_default();
            let selected = self.selection.contains(&Hit::Group(id.clone()));
            layer = layer.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-group-{id}")))
                        .absolute()
                        .left(gpui_kit::px(sr.x))
                        .top(gpui_kit::px(sr.y))
                        .w(gpui_kit::px(sr.w))
                        .h(gpui_kit::px(sr.h))
                        .rounded(gpui_kit::px(12. * s))
                        .border_1()
                        .border_dashed()
                        .border_color(if selected {
                            accent
                        } else {
                            secondary.opacity(0.6)
                        })
                        .px(gpui_kit::px(10. * s))
                        .py(gpui_kit::px(5. * s))
                        .text_size(gpui_kit::px(11. * s))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(secondary)
                        .test_support(),
                    Role::Group,
                )
                .aria_label(SharedString::from(format!("Group {name}")))
                .when(s > 0.3, |d| d.child(name)),
            );
        }

        // Arrows, with handwritten labels at their middles.
        let mut pens: Vec<Pen> = Vec::new();
        for (i, (c, e)) in layout.edges.iter().zip(&board.graph.edges).enumerate() {
            let screen = rules::Curve {
                start: cam.to_screen(c.start),
                c1: cam.to_screen(c.c1),
                c2: cam.to_screen(c.c2),
                end: cam.to_screen(c.end),
            };
            let selected = self.selection.contains(&Hit::Arrow(i));
            pens.push((
                screen.clone(),
                (if selected { 2.6 } else { 1.6 }) * s,
                if selected { accent } else { ink },
                !e.line,
            ));
            let words = e.label.clone().unwrap_or_default();
            let (mx, my) = screen.mid();
            let name = rules::hit_label(&board, &Hit::Arrow(i));
            layer = layer.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-arrow-{i}")))
                        .absolute()
                        .left(gpui_kit::px(mx - 90. * s))
                        .top(gpui_kit::px(my - 22. * s))
                        .w(gpui_kit::px(180. * s))
                        .flex()
                        .justify_center()
                        .font_family(crate::fonts::HANDWRITING)
                        .font_weight(FontWeight::BOLD)
                        .text_size(gpui_kit::px(17. * s))
                        .text_color(if selected { accent } else { ink })
                        .test_support(),
                    Role::Link,
                )
                .aria_label(SharedString::from(format!("Arrow {name}")))
                .when(s > 0.3 && !words.is_empty(), |d| d.child(words)),
            );
        }

        // Boxes, with their badges and Send to….
        let order = rules::tab_order(&board);
        for (id, r) in &layout.boxes {
            let Some(node) = board.graph.node(id).cloned() else {
                continue;
            };
            let sr = cam.rect(r);
            let status = statuses.get(id).cloned();
            let selected = self.selection.contains(&Hit::Box(id.clone()));
            let keyed = self.keyed.and_then(|k| order.get(k)) == Some(id);
            let changed = board.changed.contains(id) && self.seen_changes != Some(board.version);
            let radius = match node.shape.as_str() {
                "round" => sr.h / 2.,
                "db" => 14. * s,
                _ => 8. * s,
            };
            let a11y = rules::box_label(&board, id, status.as_ref());
            let mut b = crate::a11y::A11yRole::a11y_role(
                div()
                    .id(SharedString::from(format!("plan-box-{id}")))
                    .absolute()
                    .left(gpui_kit::px(sr.x))
                    .top(gpui_kit::px(sr.y))
                    .w(gpui_kit::px(sr.w))
                    .h(gpui_kit::px(sr.h))
                    .rounded(gpui_kit::px(radius))
                    .bg(card)
                    .border(gpui_kit::px(if selected || keyed { 2. } else { 1. }))
                    .border_color(if selected || keyed {
                        accent
                    } else {
                        status
                            .as_ref()
                            .map(|st| status_colour(st, cx))
                            .unwrap_or(edge)
                    })
                    .when(node.shape == "db", |d| d.border_b(gpui_kit::px(4. * s)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .px(gpui_kit::px(8. * s))
                    .text_size(gpui_kit::px(13. * s))
                    .text_color(label)
                    .text_center()
                    .cursor(CursorStyle::PointingHand)
                    .test_support(),
                Role::Button,
            )
            .aria_label(SharedString::from(a11y))
            .when(s > 0.2, |d| d.child(node.label.clone()));
            if changed && s > 0.3 {
                b = b.child(
                    // Clear of the box's top edge, which drew through it at -9.
                    div()
                        .absolute()
                        .top(gpui_kit::px(-18. * s))
                        .left(gpui_kit::px(8. * s))
                        .px(gpui_kit::px(5. * s))
                        .rounded(gpui_kit::px(4. * s))
                        .bg(style::needs_you(cx))
                        .text_size(gpui_kit::px(9. * s))
                        .text_color(rgb(0xFFFFFF))
                        .child("Changed"),
                );
            }
            layer = layer.child(b);
            if s > 0.35 {
                layer = layer.child(self.box_controls(&board, id, &sr, status.as_ref(), cx));
            }
        }

        layer = layer.children(note_bubble(&board, &layout, cam, cx));
        // planBoardMockups: the open box's screens, as stills.
        layer = layer.children(self.render_screens(&board, &layout, cx));

        let bounds_cell = self.bounds.clone();
        let dot: Hsla = rgb(if dark { 0x2E2E36 } else { 0xC4C4CE }).into();
        let dots = canvas(
            move |bounds, _, _| bounds_cell.set(Some(bounds)),
            move |bounds, _, window, _| paint_dots(cam, dot, bounds, window),
        )
        .absolute()
        .inset_0();
        let painter = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| paint_arrows(&pens, bounds.origin, window),
        )
        .absolute()
        .inset_0();
        div()
            .id("plan-board-canvas")
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_hidden()
            .bg(rgb(if dark { CANVAS_DARK } else { CANVAS_LIGHT }))
            .cursor(CursorStyle::OpenHand)
            .child(dots)
            .child(painter)
            .child(layer)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| this.pressed(ev, window, cx)),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| this.mouse_moved(ev, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseUpEvent, _, cx| this.released(ev, cx)),
            )
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| this.scrolled(ev, cx)))
            .on_pinch(cx.listener(|this, ev: &PinchEvent, _, cx| this.pinched(ev, cx)))
            .into_any_element()
    }
}

// ---------------------------------------------------------------- a box's controls

impl PlanBoardView {
    /// Under a box: its status badge (a click opens its thread) and Send to….
    fn box_controls(
        &mut self,
        board: &Board,
        id: &str,
        sr: &Rect,
        status: Option<&BoxStatus>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut row = h_flex()
            .absolute()
            .left(gpui_kit::px(sr.x))
            .top(gpui_kit::px(sr.bottom() + 4.))
            .gap(px(4.))
            .items_center();
        if let Some(st) = status {
            let colour = status_colour(st, cx);
            let detail = match st {
                BoxStatus::Working(line) if !line.is_empty() => format!("Working · {line}"),
                other => other.word().to_owned(),
            };
            let link = board.links.get(id).cloned();
            let target = link.as_ref().and_then(|l| l.thread_id.clone());
            let who = link
                .as_ref()
                .map(|l| {
                    if l.is_coordinator() {
                        "Coordinator"
                    } else if l.harness.as_deref() == Some("codex") {
                        "Codex"
                    } else {
                        "Claude"
                    }
                })
                .unwrap_or("");
            let tip = format!("{who} {detail}: open its thread");
            row = row.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-badge-{id}")))
                        .h(px(18.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .rounded(px(9.))
                        .bg(colour.opacity(0.15))
                        .text_size(px(size::SMALL))
                        .text_color(colour)
                        .max_w(px(220.))
                        .truncate()
                        .test_support(),
                    Role::Link,
                )
                .aria_label(SharedString::from(format!("{} says {}", id, st.word())))
                .tip(tip)
                .child(format!("{who} {detail}"))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .when_some(target, |d, t| {
                    d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open_thread(t.clone(), window, cx)
                    }))
                }),
            );
        }
        if self.mockups_on(cx) {
            let node = id.to_owned();
            let open = self.screens_for.as_deref() == Some(id);
            row = row.child(
                bar_button_dyn(
                    format!("plan-screens-{id}"),
                    "Screens…",
                    if open {
                        "Hide this box's screens"
                    } else {
                        "Show this box's screens"
                    },
                    true,
                    cx,
                )
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_screens(&node, cx)),
                ),
            );
        }
        let allowed = rules::send_allowed(board, id, status.cloned());
        let again = matches!(status, Some(BoxStatus::Failed | BoxStatus::Archived));
        let shown = match (&allowed, again) {
            (_, true) => "Send again",
            _ => "Send to…",
        };
        // Already sent and going: no button, the badge says where it is.
        if !matches!(allowed, Err("Already sent")) {
            let tip: SharedString = match allowed {
                Ok(()) => format!(
                    "{shown} {}",
                    board.graph.node(id).map(|n| n.label.as_str()).unwrap_or(id)
                )
                .into(),
                Err(why) => why.into(),
            };
            let enabled = allowed.is_ok();
            let node = id.to_owned();
            row = row.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-send-{id}")))
                        .h(px(18.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .rounded(px(9.))
                        .border_1()
                        .border_color(style::button_edge(cx))
                        .bg(style::button_fill(cx))
                        .text_size(px(size::SMALL))
                        .text_color(style::label_text(cx))
                        .when(!enabled, |d| d.opacity(0.4))
                        .test_support(),
                    Role::Button,
                )
                .aria_label(SharedString::from(format!(
                    "{shown} {id}{}",
                    if enabled { "" } else { ", disabled" }
                )))
                .tip(tip)
                .child(shown)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .when(enabled, |d| {
                    d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open_send(&node, window, cx)
                    }))
                }),
            );
        }
        row.into_any_element()
    }

    fn open_send(&mut self, node: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(board) = self.board.clone() else {
            return;
        };
        let statuses = self.statuses(cx);
        if rules::send_allowed(&board, node, statuses.get(node).cloned()).is_err() {
            return;
        }
        let app = self.app.upgrade();
        let (summary, accounts) = match &app {
            Some(a) => {
                let a = a.read(cx);
                (
                    a.threads.get(&self.thread_id).map(|t| (**t).clone()),
                    a.logins.accounts.clone(),
                )
            }
            None => (None, Vec::new()),
        };
        let Some(planner) = summary else { return };
        let me = cx.entity().downgrade();
        let coordinator = self.coordinator.clone().filter(|_| {
            app.as_ref()
                .is_some_and(|a| a.read(cx).experiment_on(rules::COORDINATORS))
        });
        let sheet = cx.new(|cx| {
            SendSheet::new(
                me,
                self.client.clone(),
                board,
                node.to_owned(),
                planner,
                accounts,
                coordinator,
                window,
                cx,
            )
        });
        #[cfg(feature = "automation")]
        SHEET.with(|o| *o.borrow_mut() = Some(sheet.downgrade()));
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .w(px(560.))
                .close_button(false)
                .child(sheet.clone())
                .footer(div())
        });
        crate::web_panels::keyboard_to_app(window);
    }
}

#[cfg(feature = "automation")]
thread_local! {
    static SHEET: std::cell::RefCell<Option<WeakEntity<SendSheet>>> = const { std::cell::RefCell::new(None) };
}

/// Send to…: the login, the access, the title, the board's skills and the
/// brief, editable, before anything starts. Start makes one request id for
/// the sheet, so pressing it twice, or a slow runner, starts one thread.
pub(crate) struct SendSheet {
    view: WeakEntity<PlanBoardView>,
    client: Arc<Client>,
    board: Board,
    node: String,
    planner: threadbear_core::inbox::ThreadSummary,
    accounts: Vec<threadbear_core::logins::Account>,
    account: Option<String>,
    mode: String,
    coordinator: Option<(String, String)>,
    to_coordinator: bool,
    title: Entity<InputState>,
    brief: Entity<TextareaState>,
    // The skills the chosen login can run, once asked.
    login_skills: Option<Vec<String>>,
    // The brief as last filled from the board: edited once it differs.
    filled: String,
    // The login's skills came in: fill the brief again at the next frame.
    refill_due: bool,
    request_id: String,
    starting: bool,
    error: Option<String>,
}

impl SendSheet {
    #[expect(
        clippy::too_many_arguments,
        reason = "one sheet, opened from one place"
    )]
    fn new(
        view: WeakEntity<PlanBoardView>,
        client: Arc<Client>,
        board: Board,
        node: String,
        planner: threadbear_core::inbox::ThreadSummary,
        accounts: Vec<threadbear_core::logins::Account>,
        coordinator: Option<(String, String)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = board
            .graph
            .node(&node)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| node.clone());
        let account = accounts
            .iter()
            .find(|a| a.id == planner.account_id)
            .or(accounts.first())
            .map(|a| a.id.clone());
        let title = cx.new(|cx| InputState::new(window, cx));
        title.update(cx, |t, cx| t.set_value(label, window, cx));
        let brief = cx.new(|cx| TextareaState::new(window, cx).auto_grow(8, 18));
        // The planner's access, but never Plan first: a box's thread is
        // there to build it (a planner in plan mode is the usual way in).
        let mode = if planner.permission_mode.is_empty() || planner.permission_mode == "plan" {
            "full".to_owned()
        } else {
            planner.permission_mode.clone()
        };
        let mut s = SendSheet {
            view,
            client,
            board,
            node,
            planner,
            accounts,
            account,
            mode,
            coordinator,
            to_coordinator: false,
            title,
            brief,
            login_skills: None,
            filled: String::new(),
            refill_due: false,
            request_id: uuid_like(),
            starting: false,
            error: None,
        };
        s.refill(window, cx);
        s.load_login_skills(cx);
        s
    }

    fn provider(&self) -> String {
        if self.to_coordinator {
            return "claude".to_owned();
        }
        self.accounts
            .iter()
            .find(|a| Some(&a.id) == self.account.as_ref())
            .map(|a| a.provider.clone())
            .unwrap_or_else(|| "claude".to_owned())
    }

    /// The skills this brief names: the board's, less any the login lacks.
    fn skills(&self) -> (Vec<String>, Vec<String>) {
        match &self.login_skills {
            Some(have) if !self.to_coordinator => rules::skills_for_login(&self.board.skills, have),
            _ => (self.board.skills.clone(), Vec::new()),
        }
    }

    /// The brief from the board, unless Wayne has edited it.
    fn edited(&self, cx: &App) -> bool {
        self.brief.read(cx).value().as_ref() != self.filled.as_str()
    }

    fn refill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edited(cx) {
            return;
        }
        let (skills, _) = self.skills();
        let text =
            rules::brief(&self.board, &self.node, &self.provider(), &skills).unwrap_or_default();
        self.filled = text.clone();
        self.brief.update(cx, |b, cx| b.set_value(text, window, cx));
    }

    fn load_login_skills(&mut self, cx: &mut Context<Self>) {
        let Some(account) = self.account.clone() else {
            return;
        };
        let client = self.client.clone();
        let params = json!({ "cwd": self.planner.cwd, "accountId": account });
        cx.spawn(async move |this, cx| {
            let res = call(client, "folders.commands", params).await;
            this.update(cx, |s, cx| {
                s.login_skills = Some(skill_names(res.ok().as_ref()));
                s.refill_due = true;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// A request id for one sheet's Start.
fn uuid_like() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("pb-{n:x}-{:x}", std::process::id())
}

impl SendSheet {
    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.starting {
            return;
        }
        self.starting = true;
        self.error = None;
        let brief = self.brief.read(cx).value().to_string();
        let title = self.title.read(cx).value().trim().to_owned();
        let thread = self.planner.id.clone();
        let mut params = json!({
            "threadId": thread,
            "nodeId": self.node,
            "requestId": self.request_id,
            "brief": brief,
            "title": title,
        });
        if self.to_coordinator {
            params["destination"] = json!("coordinator");
        } else {
            params["permissionMode"] = json!(self.mode);
            if let Some(a) = &self.account {
                params["accountId"] = json!(a);
            }
        }
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "board.send", params).await;
            this.update_in(cx, |s, window, cx| {
                s.starting = false;
                match res {
                    Ok(v) => {
                        let board = Board::decode(&v["board"]);
                        s.view
                            .update(cx, |b, cx| {
                                b.take_board(board);
                                cx.notify();
                            })
                            .ok();
                        window.close_dialog(cx);
                    }
                    Err(e) => {
                        s.error = Some(e.message);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn login_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self
            .accounts
            .iter()
            .find(|a| Some(&a.id) == self.account.as_ref())
            .map(|a| a.title.clone())
            .unwrap_or_else(|| "Default".to_owned());
        let choices: Vec<(String, String, String)> = self
            .accounts
            .iter()
            .filter(|a| a.provider == "claude" || a.provider == "codex")
            .map(|a| (a.id.clone(), a.title.clone(), a.provider.clone()))
            .collect();
        let picked = self.account.clone();
        let me = cx.entity().downgrade();
        Button::new("plan-send-login")
            .label(current)
            .small()
            .outline()
            .dropdown_caret(true)
            .disabled(self.starting || self.to_coordinator)
            .tooltip("The Claude Code or Codex login the box's thread runs on")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for (id, title, provider) in &choices {
                    let (id, me) = (id.clone(), me.clone());
                    let name = format!(
                        "{title} ({})",
                        if provider == "codex" {
                            "Codex"
                        } else {
                            "Claude"
                        }
                    );
                    menu = menu.item(
                        PopupMenuItem::new(name)
                            .checked(Some(&id) == picked.as_ref())
                            .on_click(move |_, _, cx| {
                                me.update(cx, |s, cx| {
                                    s.account = Some(id.clone());
                                    s.login_skills = None;
                                    s.load_login_skills(cx);
                                    s.refill_due = true;
                                    cx.notify();
                                })
                                .ok();
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn mode_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let name = |m: &str| match m {
            "ask" => "Ask before edits",
            "auto" => "Auto",
            "plan" => "Plan first",
            _ => "Full access",
        };
        let current = name(&self.mode).to_owned();
        let picked = self.mode.clone();
        let me = cx.entity().downgrade();
        Button::new("plan-send-mode")
            .label(current)
            .small()
            .outline()
            .dropdown_caret(true)
            .disabled(self.starting || self.to_coordinator)
            .tooltip("Its access; the planner's unless you pick another")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for m in ["full", "ask", "auto"] {
                    let me = me.clone();
                    menu = menu.item(PopupMenuItem::new(name(m)).checked(picked == m).on_click(
                        move |_, _, cx| {
                            me.update(cx, |s, cx| {
                                s.mode = m.to_owned();
                                cx.notify();
                            })
                            .ok();
                        },
                    ));
                }
                menu
            })
            .into_any_element()
    }
}

impl Render for SendSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.refill_due {
            self.refill_due = false;
            self.refill(window, cx);
        }
        let label = self
            .board
            .graph
            .node(&self.node)
            .map(|n| n.label.clone())
            .unwrap_or_default();
        let (skills, missing) = self.skills();
        let row = |name: &'static str, control: AnyElement| {
            h_flex()
                .gap(px(10.))
                .items_center()
                .child(
                    div()
                        .w(px(70.))
                        .text_size(px(size::LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name),
                )
                .child(control)
        };
        let mut body = v_flex().id("plan-send-sheet").gap(px(10.)).child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("Send “{label}” to…")),
        );
        if let Some((_, coord)) = self.coordinator.clone() {
            let on = self.to_coordinator;
            let me = cx.entity().downgrade();
            let _ = coord;
            body = body.child(
                Button::new("plan-send-destination")
                    .label(if on {
                        "Give to the project coordinator"
                    } else {
                        "A new nested thread"
                    })
                    .small()
                    .outline()
                    .dropdown_caret(true)
                    .tooltip("Where the box goes")
                    .dropdown_menu(move |menu: PopupMenu, _, _| {
                        let (a, b) = (me.clone(), me.clone());
                        menu.item(
                            PopupMenuItem::new("A new nested thread")
                                .checked(!on)
                                .on_click(move |_, _, cx| {
                                    a.update(cx, |s, cx| {
                                        s.to_coordinator = false;
                                        s.refill_due = true;
                                        cx.notify();
                                    })
                                    .ok();
                                }),
                        )
                        .item(
                            PopupMenuItem::new("Give to the project coordinator")
                                .checked(on)
                                .on_click(move |_, _, cx| {
                                    b.update(cx, |s, cx| {
                                        s.to_coordinator = true;
                                        s.refill_due = true;
                                        cx.notify();
                                    })
                                    .ok();
                                }),
                        )
                    }),
            );
            if on {
                body = body.child(div().text_size(px(size::DETAIL)).text_color(style::secondary_text(cx)).child("The coordinator runs on Claude; other providers aren't taken by coordinators yet. It gets one to-do with this brief."));
            }
        }
        if !self.to_coordinator {
            body = body
                .child(row("Login", self.login_menu(cx)))
                .child(row("Access", self.mode_menu(cx)));
        }
        body = body.child(row(
            "Title",
            div()
                .flex_1()
                .child(Input::new(&self.title).small())
                .into_any_element(),
        ));
        let skills_words = if self.board.skills.is_empty() {
            "None. Pick them with Skills on the board.".to_owned()
        } else {
            skills
                .iter()
                .map(|s| format!("/{s}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        body = body.child(row(
            "Skills",
            div()
                .text_size(px(size::DETAIL))
                .text_color(style::secondary_text(cx))
                .child(skills_words)
                .into_any_element(),
        ));
        if !missing.is_empty() {
            body = body.child(
                div()
                    .text_size(px(size::DETAIL))
                    .text_color(style::needs_you(cx))
                    .child(format!(
                        "Not on this login, so left out of the brief: {}. Add it on the Skills page, or put it in that login's skills folder.",
                        missing.iter().map(|s| format!("/{s}")).collect::<Vec<_>>().join(", ")
                    )),
            );
        }
        body.child(
            div()
                .text_size(px(size::LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Brief"),
        )
        .child(Textarea::new(&self.brief))
        .when_some(self.error.clone(), |b, e| {
            b.child(
                div()
                    .text_size(px(size::LABEL))
                    .text_color(style::failed(cx))
                    .child(e),
            )
        })
        .child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .child(div().flex_1())
                .child(
                    Button::new("plan-send-cancel")
                        .label("Cancel")
                        .small()
                        .outline()
                        .on_click(cx.listener(|_, _, window, cx| window.close_dialog(cx))),
                )
                .child(
                    Button::new("plan-send-start")
                        .label(if self.starting {
                            "Starting…"
                        } else {
                            "Start"
                        })
                        .small()
                        .primary()
                        .disabled(self.starting)
                        .on_click(cx.listener(|s, _, window, cx| s.start(window, cx))),
                ),
        )
    }
}

impl Render for PlanBoardView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let statuses = self.statuses(cx);
        let body = if self.board.is_some() {
            let bar = self.render_bar(&statuses, cx);
            let canvas = self.render_canvas(&statuses, window, cx);
            let hint = self
                .board
                .as_ref()
                .map(rules::next_step)
                .unwrap_or_default();
            v_flex()
                .size_full()
                .child(bar)
                .child(
                    crate::a11y::A11yRole::a11y_role(
                        div()
                            .id("plan-board-next")
                            .flex_none()
                            .px(px(12.))
                            .py(px(5.))
                            .text_size(px(size::DETAIL))
                            .text_color(style::quiet_text(cx))
                            .border_b_1()
                            .border_color(style::hairline(cx))
                            .test_support(),
                        Role::Label,
                    )
                    .aria_label(SharedString::from(hint))
                    .child(hint),
                )
                .child(canvas)
                .into_any_element()
        } else {
            let words = if self.loading {
                "Reading the plan…".to_owned()
            } else {
                self.error.clone().unwrap_or_else(|| {
                    "This thread has no plan yet. Say \"plan board\" in a message to start one."
                        .to_owned()
                })
            };
            crate::files::pane_note(&words, crate::highlights::palette(cx)).into_any_element()
        };
        let focus = self.focus.clone();
        div()
            .id("plan-board")
            .track_focus(&focus)
            .key_context("PlanBoard")
            .size_full()
            .child(body)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if this.key(ev, window, cx) {
                    cx.stop_propagation();
                }
            }))
    }
}

// ---------------------------------------------------------------- scripts

/// The scripted app's `board` command (features/steps/plan-board.steps.ts):
/// `{ cmd: "board", thread, do?: show | click | escape | approve | send | start | skill | review | tidy, box?, skill? }`,
/// answering the board as it's drawn.
#[cfg(feature = "automation")]
pub(crate) fn automation(
    app: &Entity<Threadbear>,
    cmd: &Value,
    window: &mut Window,
    cx: &mut App,
) -> Result<Value, String> {
    use gpui_kit::test::TestWindowExt as _;
    let thread = match cmd["thread"].as_str() {
        Some(t) => app.read(cx).thread_id_from(t)?,
        None => app
            .read(cx)
            .open_thread()
            .map(|t| t.id.clone())
            .ok_or("no thread open")?,
    };
    let action = cmd["do"].as_str().unwrap_or("state");
    if action == "show" {
        app.update(cx, |a, cx| a.show_plan(&thread, cx));
        window.render_frame(cx);
    }
    let sheet = || SHEET.with(|o| o.borrow().clone()).and_then(|w| w.upgrade());
    if action == "start" {
        let s = sheet().ok_or("no send sheet open")?;
        s.update(cx, |s, cx| {
            // Give to the project coordinator, when the sheet offers it.
            if cmd["destination"].as_str() == Some("coordinator") && s.coordinator.is_some() {
                s.to_coordinator = true;
            }
            s.start(window, cx)
        });
        return Ok(json!("ok"));
    }
    if action == "sheet" {
        let s = sheet().ok_or("no send sheet open")?;
        let s = s.read(cx);
        return Ok(
            json!({ "brief": s.brief.read(cx).value().to_string(), "coordinator": s.coordinator.is_some(), "toCoordinator": s.to_coordinator, "starting": s.starting, "error": s.error }),
        );
    }
    let view = cx
        .default_global::<PlanBoards>()
        .views
        .get(&thread)
        .cloned()
        .ok_or("the board isn't open")?;
    let label_to_id = |v: &PlanBoardView, label: &str| -> Option<String> {
        v.board
            .as_ref()?
            .graph
            .nodes
            .iter()
            .find(|n| n.label == label || n.id == label)
            .map(|n| n.id.clone())
    };
    // The board's Pop out / Put back button, pressed: Put back sits in the
    // board's own window, which `click` (the main window's) can't reach.
    if action == "toggle-window" {
        view.update(cx, |v, cx| v.pop_out_or_back(window, cx));
        return Ok(json!("ok"));
    }
    match action {
        "click" => {
            let label = cmd["box"].as_str().unwrap_or_default().to_owned();
            view.update(cx, |v, cx| match label_to_id(v, &label) {
                Some(id) => {
                    v.select(Hit::Box(id), false, cx);
                    Ok(())
                }
                None => Err(format!("no box {label:?}")),
            })?;
        }
        "escape" => {
            view.update(cx, |v, cx| v.clear(cx));
        }
        "approve" => view.update(cx, |v, cx| v.approve(cx)),
        "tidy" => view.update(cx, |v, cx| v.tidy(cx)),
        "skill" => {
            let name = cmd["skill"].as_str().unwrap_or_default().to_owned();
            view.update(cx, |v, cx| {
                let mut skills = v
                    .board
                    .as_ref()
                    .map(|b| b.skills.clone())
                    .unwrap_or_default();
                if !skills.contains(&name) {
                    skills.push(name);
                }
                v.set_skills(skills, cx);
            });
        }
        "send" => {
            let label = cmd["box"].as_str().unwrap_or_default().to_owned();
            view.update(cx, |v, cx| match label_to_id(v, &label) {
                Some(id) => {
                    v.open_send(&id, window, cx);
                    Ok(())
                }
                None => Err(format!("no box {label:?}")),
            })?;
        }
        "review" => {
            view.update(cx, |v, cx| {
                let statuses = v.statuses(cx);
                match v
                    .board
                    .as_ref()
                    .and_then(|b| rules::review_message(&statuses, b))
                {
                    Some(m) => {
                        v.ask_review(m, cx);
                        Ok(())
                    }
                    None => Err("there's no Ask planner to review".to_owned()),
                }
            })?;
        }
        "reload" => view.update(cx, |v, cx| v.reload(cx)),
        "screens" => {
            let label = cmd["box"].as_str().unwrap_or_default().to_owned();
            view.update(cx, |v, cx| match label_to_id(v, &label) {
                Some(id) if v.mockups_on(cx) => {
                    v.toggle_screens(&id, cx);
                    Ok(())
                }
                Some(_) => Err("there's no Screens… with Plan board mockups off".to_owned()),
                None => Err(format!("no box {label:?}")),
            })?;
        }
        "pick" => {
            let variant = cmd["variant"].as_str().unwrap_or_default().to_owned();
            view.update(cx, |v, cx| {
                let f = v
                    .frames
                    .iter()
                    .find(|f| Some(&f.node) == v.screens_for.as_ref() && f.variant == variant)
                    .cloned();
                match f {
                    Some(f) => {
                        v.pick_frame(&f, cx);
                        Ok(())
                    }
                    None => Err(format!("no screen {variant}")),
                }
            })?;
        }
        _ => {}
    }
    window.render_frame(cx);
    let v = view.read(cx);
    let Some(board) = v.board.clone() else {
        return Ok(json!({ "loading": v.loading, "error": v.error }));
    };
    let statuses = v.statuses(cx);
    let boxes: Vec<Value> = board
        .graph
        .nodes
        .iter()
        .map(|n| {
            let st = statuses.get(&n.id);
            json!({
                "id": n.id, "label": n.label, "group": n.group,
                "status": st.map(|s| s.word()),
                "sendEnabled": rules::send_allowed(&board, &n.id, st.cloned()).is_ok(),
                "changed": board.changed.contains(&n.id),
            })
        })
        .collect();
    let name = |id: &str| {
        board
            .graph
            .node(id)
            .map(|n| n.label.clone())
            .unwrap_or_default()
    };
    let arrows: Vec<Value> = board
        .graph
        .edges
        .iter()
        .map(|e| json!({ "from": name(&e.from), "to": name(&e.to), "label": e.label }))
        .collect();
    let groups: Vec<Value> = board.graph.groups.iter().map(|g| json!(g.label)).collect();
    Ok(json!({
        "state": board.state().words(),
        "floating": v.floating(cx),
        "version": board.version,
        "boxes": boxes,
        "groups": groups,
        "arrows": arrows,
        "skills": board.skills,
        "chips": v.chips,
        "review": rules::review_message(&statuses, &board),
        "screensButton": v.mockups_on(cx),
        "screens": v.frames.iter().filter(|f| Some(&f.node) == v.screens_for.as_ref()).map(|f| json!({
            "id": f.id, "variant": f.variant, "title": f.title, "version": f.version, "picked": f.picked, "changed": f.changed,
            "drawn": v.stills.iter().any(|((id, ver, _), p)| *id == f.id && *ver == f.version && p.is_some()),
        })).collect::<Vec<_>>(),
        "asked": v.asked,
        "error": v.error,
    }))
}

// ---------------------------------------------------------------- Phase 2: screens

impl PlanBoardView {
    fn mockups_on(&self, cx: &App) -> bool {
        self.app.upgrade().is_some_and(|a| {
            let a = a.read(cx);
            a.experiment_on(rules::EXPERIMENT) && a.experiment_on(rules::MOCKUPS)
        })
    }

    /// Screens…: show a box's screens beside it, or hide them.
    fn toggle_screens(&mut self, node: &str, cx: &mut Context<Self>) {
        if self.screens_for.as_deref() == Some(node) {
            self.screens_for = None;
            cx.notify();
            return;
        }
        self.screens_for = Some(node.to_owned());
        self.load_frames(None, cx);
        cx.notify();
    }

    /// board.frames: the screens, with a pick first when there is one.
    fn load_frames(&mut self, pick: Option<String>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let mut params = json!({ "threadId": self.thread_id, "seen": true });
        if let Some(p) = pick {
            params["pick"] = json!(p);
        }
        cx.spawn(async move |this, cx| {
            let res = call(client, "board.frames", params).await;
            this.update(cx, |this, cx| {
                match res {
                    Ok(v) => {
                        this.frames = v["frames"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|f| serde_json::from_value::<rules::Frame>(f.clone()).ok())
                            .collect();
                        this.error = None;
                    }
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A screen's still, drawn sealed at its viewport, once per version and
    /// look, one at a time, once the web runtime is up (as reply blocks are).
    fn still(
        &mut self,
        f: &rules::Frame,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> Option<Arc<Image>> {
        let key = (f.id.clone(), f.version, dark);
        if let Some(got) = self.stills.get(&key) {
            return got.clone();
        }
        if self.drawing {
            return None;
        }
        let path = f.path.clone()?;
        let runtime = crate::web_runtime::runtime(cx)?;
        if !runtime.update(cx, |r, cx| r.ensure_engine(cx)) {
            // Starting: the next frame asks again.
            if self.runtime_watch.is_none() {
                self.runtime_watch = Some(cx.observe(&runtime, |_, _, cx| cx.notify()));
            }
            return None;
        }
        self.stills.insert(key.clone(), None);
        self.drawing = true;
        let (w, h) = rules::viewport_size(&f.viewport);
        cx.spawn(async move |this, cx| {
            let html = std::fs::read_to_string(&path).unwrap_or_default();
            let png = threadbear_web::draw_page_at(&html, w as u32, h as u32, 0.5, dark).await;
            this.update(cx, |this, cx| {
                this.drawing = false;
                match png {
                    Ok(png) => {
                        this.stills.insert(
                            key,
                            Some(Arc::new(Image::from_bytes(ImageFormat::Png, png))),
                        );
                    }
                    Err(e) => this.error = Some(format!("A screen didn't draw: {e}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        None
    }

    /// Open in browser, for a route screen: its dev server page in the
    /// agents' own browser (BrowserBear's), never Wayne's signed-in one. A
    /// sketch screen stays a still: opening its file would go to the
    /// default browser, which may be his own.
    fn open_frame(&mut self, f: &rules::Frame, cx: &mut Context<Self>) {
        let Some(url) = f.route.clone() else { return };
        match crate::agent_browser::model(cx) {
            Some(b) => {
                let thread = self.thread_id.clone();
                b.update(cx, |b, cx| b.open(&url, &thread, cx));
            }
            None => {
                self.error = Some(
                    "The agents' browser isn't running, so the screen wasn't opened.".to_owned(),
                );
                cx.notify();
            }
        }
    }

    /// The strip beside the box: each screen's still, its title, letter,
    /// size and version, Pick and Open in browser. Pictures only: a screen's
    /// html is never put in the board as markup.
    fn render_screens(
        &mut self,
        board: &Board,
        layout: &rules::Layout,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(node) = self.screens_for.clone() else {
            return Vec::new();
        };
        let Some(r) = layout.rect(&node) else {
            return Vec::new();
        };
        let mine: Vec<rules::Frame> = self
            .frames
            .iter()
            .filter(|f| f.node == node)
            .cloned()
            .collect();
        let cam = self.camera;
        let s = cam.s;
        let dark = cx.theme().is_dark();
        if mine.is_empty() {
            let sr = cam.rect(&r);
            return vec![div()
                .absolute()
                .left(gpui_kit::px(sr.right() + 16.))
                .top(gpui_kit::px(sr.y))
                .text_size(px(size::DETAIL))
                .text_color(style::quiet_text(cx))
                .child("No screens yet. Click the box, then type in the message box: \"make three takes on this screen\".")
                .into_any_element()];
        }
        let mut out = Vec::new();
        for (id, rect) in rules::frame_strip(&r, &mine, board.graph.top_to_bottom()) {
            let Some(f) = mine.iter().find(|f| f.id == id).cloned() else {
                continue;
            };
            let sr = cam.rect(&rect);
            let picture = self.still(&f, dark, cx);
            let edge = if f.picked {
                rgb(GREEN).into()
            } else {
                style::hairline(cx)
            };
            let words = format!(
                "{} · {} · {} · v{}",
                f.title, f.variant, f.viewport, f.version
            );
            let chip = rules::frame_chip(board, &f);
            let (pick_f, open_f) = (f.clone(), f.clone());
            out.push(
                v_flex()
                    .absolute()
                    .left(gpui_kit::px(sr.x))
                    .top(gpui_kit::px(sr.y - 18.))
                    .w(gpui_kit::px(sr.w))
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .gap(px(4.))
                            .text_size(px(size::SMALL))
                            .text_color(style::secondary_text(cx))
                            .child(div().truncate().child(words.clone()))
                            .when(f.changed, |d| {
                                d.child(div().text_color(style::needs_you(cx)).child("Changed"))
                            }),
                    )
                    .child(
                        crate::a11y::A11yRole::a11y_role(
                            div()
                                .id(SharedString::from(format!("plan-screen-{id}")))
                                .w(gpui_kit::px(sr.w))
                                .h(gpui_kit::px(sr.h))
                                .rounded(px(6.))
                                .overflow_hidden()
                                .border(px(if f.picked { 3. } else { 1. }))
                                .border_color(edge)
                                .bg(style::card_background(cx))
                                .test_support(),
                            Role::Image,
                        )
                        .aria_label(SharedString::from(format!(
                            "Screen {words}{}",
                            if f.picked { ", picked" } else { "" }
                        )))
                        .tip("Put this screen in the message box")
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.chip_text(chip.clone(), cx)
                        }))
                        .children(picture.map(|p| img(ImageSource::Image(p)).size_full()))
                        .when(
                            s > 0.2
                                && self
                                    .stills
                                    .get(&(f.id.clone(), f.version, dark))
                                    .is_some_and(Option::is_none),
                            |d| {
                                d.child(
                                    div()
                                        .p(px(6.))
                                        .text_size(px(size::SMALL))
                                        .text_color(style::quiet_text(cx))
                                        .child(if f.route.is_some() {
                                            "A page on the dev server: Open in browser"
                                        } else {
                                            "Drawing…"
                                        }),
                                )
                            },
                        ),
                    )
                    .child(
                        h_flex()
                            .gap(px(4.))
                            .child(
                                bar_button_dyn(
                                    format!("plan-pick-{id}"),
                                    "Pick",
                                    "Pick this take",
                                    !f.picked,
                                    cx,
                                )
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(cx.listener(
                                    move |this, _: &ClickEvent, _, cx| this.pick_frame(&pick_f, cx),
                                )),
                            )
                            .when(f.route.is_some(), |r| {
                                r.child(
                                    bar_button_dyn(
                                        format!("plan-open-{id}"),
                                        "Open in browser",
                                        "Open it in the agents' browser",
                                        true,
                                        cx,
                                    )
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _, cx| {
                                            this.open_frame(&open_f, cx)
                                        },
                                    )),
                                )
                            }),
                    )
                    .into_any_element(),
            );
        }
        out
    }

    fn pick_frame(&mut self, f: &rules::Frame, cx: &mut Context<Self>) {
        let Some(board) = &self.board else { return };
        let chip = rules::frame_chip(board, f);
        for x in self
            .frames
            .iter_mut()
            .filter(|x| x.node == f.node && x.frame == f.frame)
        {
            x.picked = x.id == f.id;
        }
        self.chip_text(chip, cx);
        self.load_frames(Some(f.id.clone()), cx);
    }

    /// Any chip text into the thread's message box.
    fn chip_text(&mut self, reference: String, cx: &mut Context<Self>) {
        if !self.chips.contains(&reference) {
            self.chips.push(reference.clone());
        }
        if let Some(app) = self.app.upgrade() {
            let composer = app.read(cx).composers.get(&self.thread_id).cloned();
            if let Some(c) = composer {
                c.update(cx, |c, cx| c.mention_reference(reference, cx));
            }
        }
        cx.notify();
    }
}

/// `bar_button` with an id made at run time (one per screen).
fn bar_button_dyn(
    id: String,
    label: &'static str,
    tip: &'static str,
    enabled: bool,
    cx: &App,
) -> impl StatefulInteractiveElement + ParentElement + IntoElement {
    let tip: SharedString = tip.into();
    crate::a11y::A11yRole::a11y_role(
        div()
            .id(SharedString::from(id))
            .h(px(20.))
            .px(px(6.))
            .flex()
            .items_center()
            .rounded(px(5.))
            .border_1()
            .border_color(style::button_edge(cx))
            .bg(style::button_fill(cx))
            .text_size(px(size::SMALL))
            .text_color(style::label_text(cx))
            .when(!enabled, |d| d.opacity(0.4))
            .test_support(),
        Role::Button,
    )
    .aria_label(tip.clone())
    .tip(tip)
    .child(label)
}
