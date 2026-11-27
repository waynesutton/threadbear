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

/// A `board.updated` or `update_frame` tool event from the runner: the
/// board's tab, if open, reads the board or its screens again right away.
/// Called from the app's update, so it reads no app state.
pub(crate) fn receive_event(params: &Value, cx: &mut App) {
    let ev_type = params["event"]["type"].as_str().unwrap_or("");
    let is_board = ev_type == "board.updated";
    let is_frame_tool = matches!(ev_type, "tool.completed" | "tool.started")
        && params["event"]["name"]
            .as_str()
            .is_some_and(|n| n.ends_with("update_frame"));
    if !is_board && !is_frame_tool {
        return;
    }
    let Some(thread) = params["threadId"].as_str() else {
        return;
    };
    if !cx.has_global::<PlanBoards>() {
        return;
    }
    let boards = cx.global_mut::<PlanBoards>();
    if is_board {
        if let Some(e) = boards.has.get_mut(thread) {
            e.1 = true;
        } else {
            boards.has.insert(thread.to_owned(), (0, true));
        }
    }
    let view = boards.views.get(thread).cloned();
    if let Some(view) = view {
        if is_board {
            let board = Board::decode(&params["event"]["board"]);
            view.update(cx, |v, cx| v.agent_wrote(board, cx));
        } else {
            view.update(cx, |v, cx| {
                if v.board.is_some() && v.mockups_on(cx) {
                    v.load_frames_opts(None, false, cx);
                }
            });
        }
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
                let planner_busy = self
                    .threads
                    .get(&thread)
                    .is_some_and(|t| t.busy() || t.working());
                let v = cx.new(|cx| {
                    PlanBoardView::new(app, client, thread.clone(), dock, planner_busy, cx)
                });
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
    // Planner note collapsed for this version; a newer version shows its note again.
    note_hidden_version: Option<u32>,
    // planBoardMockups: the box whose screens are showing, its screens, and
    // their stills by (screen, version, dark).
    screens_for: Option<String>,
    frames: Vec<rules::Frame>,
    frames_loaded: bool,
    // Box currently asking the agent to generate 3 screen takes (prevents double-send).
    asking_screens_for: Option<String>,
    // Live Activity tray showing where each sent box or planner request is running.
    activity_open: bool,
    // Optimistic note right after a board action (Generate 3 takes / Ask planner to review).
    planner_activity: Option<String>,
    // Tracks planner busy -> idle so new screens load automatically without manual Refresh.
    planner_was_busy: bool,
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
        planner_was_busy: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        // Pop out and Put back follow where the tab is.
        let observe_dock = cx.observe(&dock, |_, _, cx| cx.notify());
        // Badges and the Activity tray follow the threads' live summaries; when
        // the planner finishes a turn, auto-refresh screens so new takes appear.
        let observe = app.upgrade().map(|a| {
            cx.observe(&a, |this, a, cx| {
                let busy = a
                    .read(cx)
                    .threads
                    .get(&this.thread_id)
                    .is_some_and(|t| t.busy() || t.working());
                if this.planner_was_busy && !busy {
                    this.asking_screens_for = None;
                    this.asked = None;
                    this.planner_activity = None;
                    if this.board.is_some() && this.mockups_on(cx) {
                        this.load_frames_opts(None, false, cx);
                    }
                }
                this.planner_was_busy = busy;
                cx.notify();
            })
        });
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
            note_hidden_version: None,
            screens_for: None,
            frames: Vec::new(),
            frames_loaded: false,
            asking_screens_for: None,
            activity_open: false,
            planner_activity: None,
            planner_was_busy,
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
                        if this.board.is_some() && this.mockups_on(cx) {
                            this.load_frames_opts(None, false, cx);
                        }
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
        self.asking_screens_for = None;
        self.planner_activity = None;
        self.reload(cx);
        cx.notify();
    }

    fn activity_rows(
        &self,
        statuses: &BTreeMap<String, BoxStatus>,
        cx: &App,
    ) -> Vec<rules::ActivityRow> {
        let Some(board) = &self.board else {
            return Vec::new();
        };
        let app = self.app.upgrade();
        let threads = app.as_ref().map(|a| &a.read(cx).threads);
        let planner = threads
            .and_then(|t| t.get(&self.thread_id))
            .map(|t| (**t).clone());
        rules::activity_rows(
            board,
            planner.as_ref(),
            self.planner_activity.as_deref(),
            statuses,
            |id| threads.and_then(|t| t.get(id)).map(|t| (**t).clone()),
        )
    }

    /// Called after SendSheet starts a box thread: update board links and open
    /// the live Activity tray so Wayne immediately sees where it is running.
    fn on_box_sent(&mut self, board: Option<Board>, cx: &mut Context<Self>) {
        self.take_board(board);
        self.activity_open = true;
        cx.notify();
    }

    /// Center the canvas on a box and highlight it (used from the Activity tray).
    fn focus_box(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(board) = &self.board else { return };
        let layout = rules::layout(&board.graph, &self.positions());
        if let Some(r) = layout.rect(id) {
            let (w, h) = self.view_size();
            let s = self.camera.s.max(0.85);
            let cx_world = r.x + r.w / 2.;
            let cy_world = r.y + r.h / 2.;
            self.camera = Camera {
                s,
                tx: w / 2. - cx_world * s,
                ty: h / 2. - cy_world * s,
            };
        }
        self.selection = vec![Hit::Box(id.to_owned())];
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
        self.planner_activity = Some("Asking planner to review the plan".to_owned());
        self.activity_open = true;
        let client = self.client.clone();
        let params = json!({ "threadId": self.thread_id, "text": message });
        cx.spawn(async move |this, cx| {
            if let Err(e) = call(client, "threads.send", params).await {
                this.update(cx, |this, cx| {
                    this.asked = None;
                    this.planner_activity = None;
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
            .flex_none()
            .h(px(24.))
            .px(px(8.))
            .flex()
            .items_center()
            .whitespace_nowrap()
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
    fn render_approve_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let accent = style::accent(cx);
        let enabled = !self.approving;
        crate::a11y::A11yRole::a11y_role(
            div()
                .id("plan-board-approve")
                .flex_none()
                .h(px(24.))
                .px(px(10.))
                .flex()
                .items_center()
                .whitespace_nowrap()
                .rounded(px(6.))
                .bg(accent)
                .text_size(px(size::DETAIL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(gpui_kit::white())
                .when(!enabled, |d| d.opacity(0.5))
                .when(enabled, |d| {
                    d.cursor(CursorStyle::PointingHand)
                        .hover(|s| s.opacity(0.9))
                })
                .test_support(),
            Role::Button,
        )
        .aria_label("Approve plan")
        .tip("Approve plan: unlocks Send on boxes so you can start building")
        .child(if self.approving {
            "Approving…"
        } else {
            "Approve plan"
        })
        .when(enabled, |d| {
            d.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.approve(cx)))
        })
        .into_any_element()
    }

    fn render_activity_pill(
        &self,
        summary: &rules::ActivitySummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dot_colour = if summary.needs_you > 0 {
            style::needs_you(cx)
        } else if summary.running > 0 {
            style::accent(cx)
        } else if summary.failed > 0 {
            style::failed(cx)
        } else if summary.done > 0 {
            rgb(GREEN).into()
        } else {
            style::secondary_text(cx)
        };
        let open = self.activity_open;
        let label = summary.label.clone();
        crate::a11y::A11yRole::a11y_role(
            div()
                .id("plan-board-activity")
                .flex_none()
                .h(px(24.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .rounded(px(6.))
                .border_1()
                .border_color(if open {
                    dot_colour.opacity(0.85)
                } else {
                    dot_colour.opacity(0.45)
                })
                .bg(dot_colour.opacity(if open { 0.16 } else { 0.08 }))
                .text_size(px(size::DETAIL))
                .font_weight(FontWeight::MEDIUM)
                .text_color(dot_colour)
                .cursor(CursorStyle::PointingHand)
                .hover(|s| s.bg(dot_colour.opacity(0.2)))
                .test_support(),
            Role::Button,
        )
        .aria_label(SharedString::from(format!("Activity: {label}")))
        .tip("Show or hide what is running and where each box thread went")
        .child(
            div()
                .flex_none()
                .w(px(6.))
                .h(px(6.))
                .rounded_full()
                .bg(dot_colour),
        )
        .child(label)
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.activity_open = !this.activity_open;
            cx.notify();
        }))
        .into_any_element()
    }

    fn render_activity_row(
        &self,
        index: usize,
        row: &rules::ActivityRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colour = status_colour(&row.status, cx);
        let where_text = if !row.thread_title.is_empty() && row.thread_title != row.label {
            format!("{} · {}", row.where_name, row.thread_title)
        } else {
            row.where_name.clone()
        };
        let target_el: AnyElement = if let Some(node_id) = row.node_id.clone() {
            let focus_id = node_id.clone();
            crate::a11y::A11yRole::a11y_role(
                div()
                    .id(SharedString::from(format!("plan-activity-focus-{node_id}")))
                    .flex_none()
                    .max_w(px(170.))
                    .truncate()
                    .text_size(px(size::DETAIL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(style::label_text(cx))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|s| s.text_color(style::accent(cx)))
                    .test_support(),
                Role::Button,
            )
            .aria_label(SharedString::from(format!("Center box {}", row.label)))
            .tip("Center this box on the board")
            .child(row.label.clone())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.focus_box(&focus_id, cx);
            }))
            .into_any_element()
        } else {
            div()
                .flex_none()
                .text_size(px(size::DETAIL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(style::label_text(cx))
                .child(row.label.clone())
                .into_any_element()
        };
        let mut line = h_flex()
            .id(SharedString::from(format!("plan-activity-row-{index}")))
            .w_full()
            .min_h(px(28.))
            .px(px(12.))
            .py(px(4.))
            .gap(px(8.))
            .items_center()
            .border_t_1()
            .border_color(style::hairline(cx).opacity(0.6))
            .child(
                h_flex()
                    .flex_none()
                    .w(px(88.))
                    .gap(px(5.))
                    .items_center()
                    .child(
                        div()
                            .flex_none()
                            .w(px(6.))
                            .h(px(6.))
                            .rounded_full()
                            .bg(colour),
                    )
                    .child(
                        div()
                            .whitespace_nowrap()
                            .text_size(px(size::SMALL))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colour)
                            .child(row.status.word()),
                    ),
            )
            .child(target_el)
            .child(
                div()
                    .flex_none()
                    .max_w(px(200.))
                    .px(px(6.))
                    .py(px(1.))
                    .truncate()
                    .rounded(px(4.))
                    .bg(style::button_fill(cx))
                    .border_1()
                    .border_color(style::button_edge(cx))
                    .text_size(px(size::SMALL))
                    .text_color(style::secondary_text(cx))
                    .child(where_text),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
                    .text_size(px(size::SMALL))
                    .text_color(style::quiet_text(cx))
                    .child(row.detail.clone()),
            );
        if let Some(tid) = row.thread_id.clone() {
            line = line.child(
                bar_button_dyn(
                    format!("plan-activity-open-{index}"),
                    "Open thread",
                    "Open this thread",
                    true,
                    cx,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.open_thread(tid.clone(), window, cx);
                })),
            );
        }
        line.into_any_element()
    }

    fn render_activity_tray(
        &self,
        rows: &[rules::ActivityRow],
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.activity_open || rows.is_empty() {
            return None;
        }
        let summary = rules::activity_summary(rows)?;
        let header = h_flex()
            .id("plan-activity-header")
            .w_full()
            .px(px(12.))
            .py(px(5.))
            .gap(px(8.))
            .items_center()
            .child(
                div()
                    .text_size(px(size::SMALL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(style::label_text(cx))
                    .child("Running & sent"),
            )
            .child(
                div()
                    .text_size(px(size::SMALL))
                    .text_color(style::secondary_text(cx))
                    .child(summary.label),
            )
            .child(div().flex_1())
            .child(
                bar_button("plan-activity-close", "✕", "Hide activity tray", true, cx).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.activity_open = false;
                        cx.notify();
                    }),
                ),
            );
        let mut list = v_flex()
            .id("plan-activity-list")
            .w_full()
            .max_h(px(164.))
            .overflow_y_scroll();
        for (i, row) in rows.iter().enumerate() {
            list = list.child(self.render_activity_row(i, row, cx));
        }
        Some(
            v_flex()
                .id("plan-board-activity-tray")
                .flex_none()
                .w_full()
                .bg(style::card_background(cx))
                .border_b_1()
                .border_color(style::hairline(cx))
                .child(header)
                .child(list)
                .into_any_element(),
        )
    }

    fn render_bar(
        &mut self,
        statuses: &BTreeMap<String, BoxStatus>,
        activity: &[rules::ActivityRow],
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
                        .flex_none()
                        .h(px(24.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .rounded(px(6.))
                        .bg(state_colour.opacity(0.14))
                        .text_size(px(size::DETAIL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(state_colour)
                        .test_support(),
                    Role::Label,
                )
                .aria_label(SharedString::from(words.clone()))
                .child(words),
            );
        if !state.approved() {
            bar = bar.child(self.render_approve_button(cx));
        }
        if let Some(summary) = rules::activity_summary(activity) {
            bar = bar.child(self.render_activity_pill(&summary, cx));
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

/// The agent's note, pinned cleanly at the bottom left of the canvas so pan
/// and zoom never slice it off or shrink it below reading size.
fn note_hud(board: &Board, hidden: bool, cx: &mut Context<PlanBoardView>) -> Option<AnyElement> {
    let note = board.note.clone().filter(|n| !n.text.is_empty())?;
    let version = board.version;
    if hidden {
        return Some(
            crate::a11y::A11yRole::a11y_role(
                div()
                    .id("plan-note-show")
                    .absolute()
                    .left(px(12.))
                    .bottom(px(12.))
                    .h(px(26.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .rounded(px(8.))
                    .bg(style::card_background(cx))
                    .border_1()
                    .border_color(style::card_border(cx))
                    .shadow_md()
                    .text_size(px(size::DETAIL))
                    .text_color(style::secondary_text(cx))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|s| s.bg(style::hover_fill(cx)))
                    .test_support(),
                Role::Button,
            )
            .aria_label("Show planner note")
            .tip("Show planner note")
            .child("Planner note")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.note_hidden_version = None;
                cx.notify();
            }))
            .into_any_element(),
        );
    }
    let about = note
        .near
        .as_deref()
        .and_then(|id| board.graph.node(id))
        .map(|n| format!("Planner note · {}", n.label))
        .unwrap_or_else(|| "Planner note".to_owned());
    Some(
        crate::a11y::A11yRole::a11y_role(
            v_flex()
                .id("plan-note")
                .absolute()
                .left(px(12.))
                .bottom(px(12.))
                .max_w(px(360.))
                .p(px(10.))
                .gap(px(4.))
                .rounded(px(10.))
                .bg(style::card_background(cx))
                .border_1()
                .border_color(style::card_border(cx))
                .shadow_md()
                .test_support(),
            Role::Note,
        )
        .aria_label(SharedString::from(format!("Note: {}", note.text)))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(size::SMALL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(style::secondary_text(cx))
                        .child(about),
                )
                .child(
                    crate::a11y::A11yRole::a11y_role(
                        div()
                            .id("plan-note-hide")
                            .flex_none()
                            .w(px(18.))
                            .h(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.))
                            .text_size(px(size::SMALL))
                            .text_color(style::quiet_text(cx))
                            .cursor(CursorStyle::PointingHand)
                            .hover(|s| s.bg(style::hover_fill(cx)))
                            .test_support(),
                        Role::Button,
                    )
                    .aria_label("Hide planner note")
                    .tip("Hide planner note")
                    .child("✕")
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.note_hidden_version = Some(version);
                            cx.notify();
                        },
                    )),
                ),
        )
        .child(
            div()
                .text_size(px(size::DETAIL))
                .text_color(style::label_text(cx))
                .child(note.text),
        )
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
                layer = layer.child(self.box_controls(&board, id, &sr, s, status.as_ref(), cx));
            }
        }

        let note_overlay = note_hud(&board, self.note_hidden_version == Some(board.version), cx);

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
            .children(note_overlay)
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
    /// Status badge pill under a sent box: compact label + live activity tooltip.
    fn box_status_badge(
        &self,
        board: &Board,
        id: &str,
        st: &BoxStatus,
        ui_s: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pill_h = 20. * ui_s;
        let pill_px = 7. * ui_s;
        let pill_r = 6. * ui_s;
        let pill_text = 10.5 * ui_s;
        let dot_sz = (5. * ui_s).max(4.);
        let colour = status_colour(st, cx);
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
            .unwrap_or("Thread");
        // Keep the canvas pill short so it never clips mid-word; put the
        // live activity line in the tooltip instead.
        let pill_label = format!("{who} · {}", st.word());
        let tip = match st {
            BoxStatus::Working(line) if !line.is_empty() => {
                format!("{who} Working: {line} (click to open thread)")
            }
            other => format!("{who} {}: click to open thread", other.word()),
        };
        crate::a11y::A11yRole::a11y_role(
            div()
                .id(SharedString::from(format!("plan-badge-{id}")))
                .flex_none()
                .h(gpui_kit::px(pill_h))
                .px(gpui_kit::px(pill_px))
                .flex()
                .items_center()
                .gap(gpui_kit::px(4. * ui_s))
                .whitespace_nowrap()
                .rounded(gpui_kit::px(pill_r))
                .bg(style::card_background(cx))
                .border_1()
                .border_color(colour.opacity(0.55))
                .text_size(gpui_kit::px(pill_text))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colour)
                .cursor(CursorStyle::PointingHand)
                .test_support(),
            Role::Link,
        )
        .aria_label(SharedString::from(format!("{} says {}", id, st.word())))
        .tip(tip)
        .child(
            div()
                .flex_none()
                .w(gpui_kit::px(dot_sz))
                .h(gpui_kit::px(dot_sz))
                .rounded_full()
                .bg(colour),
        )
        .child(pill_label)
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .when_some(target, |d, t| {
            d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.open_thread(t.clone(), window, cx)
            }))
        })
        .into_any_element()
    }

    /// Under a box: its status badge (a click opens its thread), Screens, and Send.
    fn box_controls(
        &mut self,
        board: &Board,
        id: &str,
        sr: &Rect,
        scale: f32,
        status: Option<&BoxStatus>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Scale pills gently with canvas zoom so zoomed-out boxes aren't dwarfed
        // by giant buttons, while keeping labels crisp and never truncated.
        let ui_s = scale.clamp(0.72, 1.12);
        let pill_h = 20. * ui_s;
        let pill_px = 7. * ui_s;
        let pill_r = 6. * ui_s;
        let pill_text = 10.5 * ui_s;
        let accent = style::accent(cx);
        let mut row = h_flex()
            .absolute()
            .left(gpui_kit::px(sr.x))
            .top(gpui_kit::px(sr.bottom() + 5. * ui_s))
            .gap(gpui_kit::px(4. * ui_s))
            .items_center();
        if let Some(st) = status {
            row = row.child(self.box_status_badge(board, id, st, ui_s, cx));
        }
        if self.mockups_on(cx) {
            let node = id.to_owned();
            let open = self.screens_for.as_deref() == Some(id);
            let box_frames: Vec<&rules::Frame> =
                self.frames.iter().filter(|f| f.node == id).collect();
            let count = box_frames.len();
            let picked = box_frames.iter().find(|f| f.picked);
            let any_changed = box_frames.iter().any(|f| f.changed);
            let screens_label = if let Some(p) = picked {
                format!("Screens · {}", p.variant)
            } else if count > 0 {
                format!("Screens ({count})")
            } else {
                "Screens".to_owned()
            };
            let green: Hsla = rgb(GREEN).into();
            let border = if open {
                accent
            } else if picked.is_some() {
                green.opacity(0.65)
            } else {
                style::button_edge(cx)
            };
            let bg = if open {
                accent.opacity(0.16)
            } else {
                style::button_fill(cx)
            };
            let text_col = if open {
                accent
            } else if picked.is_some() {
                green
            } else {
                style::label_text(cx)
            };
            let tip = if open {
                "Hide this box's screens"
            } else if count > 0 {
                "Open this box's screens to compare or pick a take"
            } else {
                "Open or generate screen mockups (works before approving)"
            };
            let dot_sz = (5. * ui_s).max(4.);
            row = row.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-screens-{id}")))
                        .flex_none()
                        .h(gpui_kit::px(pill_h))
                        .px(gpui_kit::px(pill_px))
                        .flex()
                        .items_center()
                        .gap(gpui_kit::px(4. * ui_s))
                        .whitespace_nowrap()
                        .rounded(gpui_kit::px(pill_r))
                        .border_1()
                        .border_color(border)
                        .bg(bg)
                        .text_size(gpui_kit::px(pill_text))
                        .font_weight(if open || picked.is_some() {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(text_col)
                        .cursor(CursorStyle::PointingHand)
                        .hover(|s| s.bg(style::hover_fill(cx)))
                        .test_support(),
                    Role::Button,
                )
                .aria_label(tip)
                .tip(tip)
                .when(any_changed, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .w(gpui_kit::px(dot_sz))
                            .h(gpui_kit::px(dot_sz))
                            .rounded_full()
                            .bg(style::needs_you(cx)),
                    )
                })
                .child(screens_label)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_screens(&node, cx)),
                ),
            );
        }
        let allowed = rules::send_allowed(board, id, status.cloned());
        let again = matches!(status, Some(BoxStatus::Failed | BoxStatus::Archived));
        let shown = if again { "Send again" } else { "Send" };
        // Already sent and going: no button, the badge says where it is.
        if !matches!(allowed, Err("Already sent")) {
            let tip: SharedString = match allowed {
                Ok(()) => format!(
                    "Send {} to its own Claude or Codex build thread",
                    board.graph.node(id).map(|n| n.label.as_str()).unwrap_or(id)
                )
                .into(),
                Err(why) => format!("{why} to send this box to a build thread").into(),
            };
            let enabled = allowed.is_ok();
            let node = id.to_owned();
            row = row.child(
                crate::a11y::A11yRole::a11y_role(
                    div()
                        .id(SharedString::from(format!("plan-send-{id}")))
                        .flex_none()
                        .h(gpui_kit::px(pill_h))
                        .px(gpui_kit::px(pill_px))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .rounded(gpui_kit::px(pill_r))
                        .border_1()
                        .border_color(style::button_edge(cx))
                        .bg(style::button_fill(cx))
                        .text_size(gpui_kit::px(pill_text))
                        .text_color(style::label_text(cx))
                        .when(!enabled, |d| d.opacity(0.42))
                        .when(enabled, |d| {
                            d.cursor(CursorStyle::PointingHand)
                                .hover(|s| s.bg(style::hover_fill(cx)))
                        })
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
        let picked_screens: Vec<rules::Frame> = self
            .frames
            .iter()
            .filter(|f| f.node == node && f.picked)
            .cloned()
            .collect();
        let sheet = cx.new(|cx| {
            SendSheet::new(
                me,
                self.client.clone(),
                board,
                node.to_owned(),
                planner,
                accounts,
                coordinator,
                picked_screens,
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
    picked_screens: Vec<rules::Frame>,
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
        picked_screens: Vec<rules::Frame>,
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
            picked_screens,
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
        let text = rules::brief_with_screens(
            &self.board,
            &self.node,
            &self.provider(),
            &skills,
            &self.picked_screens,
        )
        .unwrap_or_default();
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
                                b.on_box_sent(board, cx);
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
        let body = if let Some(board) = self.board.clone() {
            let activity = self.activity_rows(&statuses, cx);
            let bar = self.render_bar(&statuses, &activity, cx);
            let activity_tray = self.render_activity_tray(&activity, cx);
            let canvas = self.render_canvas(&statuses, window, cx);
            let screens_drawer = self.render_screens_drawer(&board, cx);
            let hint = rules::next_step(&board);
            v_flex()
                .size_full()
                .child(bar)
                .child(
                    crate::a11y::A11yRole::a11y_role(
                        div()
                            .id("plan-board-next")
                            .flex_none()
                            .px(px(12.))
                            .py(px(6.))
                            .bg(style::card_background(cx).opacity(0.55))
                            .text_size(px(size::DETAIL))
                            .text_color(style::secondary_text(cx))
                            .border_b_1()
                            .border_color(style::hairline(cx))
                            .test_support(),
                        Role::Label,
                    )
                    .aria_label(SharedString::from(hint))
                    .child(hint),
                )
                .children(activity_tray)
                .child(canvas)
                .children(screens_drawer)
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
                    v.screens_for = Some(id);
                    v.load_frames_opts(None, true, cx);
                    cx.notify();
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
    let activity: Vec<Value> = v
        .activity_rows(&statuses, cx)
        .into_iter()
        .map(|r| {
            json!({
                "nodeId": r.node_id,
                "label": r.label,
                "where": r.where_name,
                "threadId": r.thread_id,
                "threadTitle": r.thread_title,
                "status": r.status.word(),
                "detail": r.detail,
            })
        })
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
        "activityOpen": v.activity_open,
        "activity": activity,
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

    /// Screens: show a box's screens in the bottom tray, or hide them.
    fn toggle_screens(&mut self, node: &str, cx: &mut Context<Self>) {
        if self.screens_for.as_deref() == Some(node) {
            self.screens_for = None;
            cx.notify();
            return;
        }
        self.screens_for = Some(node.to_owned());
        self.load_frames_opts(None, true, cx);
        cx.notify();
    }

    /// board.frames: the screens, with an optional pick and seen flag.
    fn load_frames_opts(&mut self, pick: Option<String>, seen: bool, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let mut params = json!({ "threadId": self.thread_id, "seen": seen });
        if let Some(p) = pick {
            params["pick"] = json!(p);
        }
        cx.spawn(async move |this, cx| {
            let res = call(client, "board.frames", params).await;
            this.update(cx, |this, cx| {
                let first_load = !this.frames_loaded;
                this.frames_loaded = true;
                match res {
                    Ok(v) => {
                        this.frames = v["frames"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|f| serde_json::from_value::<rules::Frame>(f.clone()).ok())
                            .collect();
                        if this.screens_for.is_none() {
                            if let Some(asked_node) = this.asking_screens_for.clone() {
                                if this.frames.iter().any(|f| f.node == asked_node) {
                                    this.screens_for = Some(asked_node);
                                }
                            } else if first_load {
                                if let Some(first_node) =
                                    this.frames.first().map(|f| f.node.clone())
                                {
                                    this.screens_for = Some(first_node);
                                }
                            }
                        }
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

    /// One-click request for 3 screen takes (A, B, C) for this box.
    fn ask_screens(&mut self, node: &str, cx: &mut Context<Self>) {
        if self.asking_screens_for.as_deref() == Some(node) {
            return;
        }
        let Some(board) = &self.board else { return };
        let label = board
            .graph
            .node(node)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| node.to_owned());
        let message = rules::ask_screens_message(board, node);
        self.asking_screens_for = Some(node.to_owned());
        self.planner_activity = Some(format!("Generating 3 screen takes for {label}"));
        self.activity_open = true;
        let client = self.client.clone();
        let params = json!({ "threadId": self.thread_id, "text": message });
        cx.spawn(async move |this, cx| {
            if let Err(e) = call(client, "threads.send", params).await {
                this.update(cx, |this, cx| {
                    this.asking_screens_for = None;
                    this.planner_activity = None;
                    this.error = Some(e.message);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
        cx.notify();
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

    /// Docked bottom drawer for the selected box's screens: never overlaps
    /// arrows or boxes on the canvas, always readable at 100% UI scale.
    #[expect(
        clippy::too_many_lines,
        reason = "one screens tray drawn top to bottom: header, empty or cards"
    )]
    fn render_screens_drawer(
        &mut self,
        board: &Board,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.mockups_on(cx) {
            return None;
        }
        let node = self.screens_for.clone()?;
        let box_label = board
            .graph
            .node(&node)
            .map(|n| n.label.clone())
            .unwrap_or_else(|| node.clone());
        let mine: Vec<rules::Frame> = self
            .frames
            .iter()
            .filter(|f| f.node == node)
            .cloned()
            .collect();
        let dark = cx.theme().is_dark();
        let accent = style::accent(cx);
        let green: Hsla = rgb(GREEN).into();
        let picked_variant = mine.iter().find(|f| f.picked).map(|f| f.variant.clone());
        let asking = self.asking_screens_for.as_deref() == Some(node.as_str());
        let node_for_ask = node.clone();
        let node_for_mention = node.clone();
        let mut boxes_with_screens: Vec<(String, String, usize, Option<String>)> = Vec::new();
        for n in &board.graph.nodes {
            let count = self.frames.iter().filter(|f| f.node == n.id).count();
            if count > 0 || n.id == node {
                let picked = self
                    .frames
                    .iter()
                    .find(|f| f.node == n.id && f.picked)
                    .map(|f| f.variant.clone());
                boxes_with_screens.push((n.id.clone(), n.label.clone(), count, picked));
            }
        }

        let mut header = h_flex()
            .flex_none()
            .px(px(12.))
            .py(px(8.))
            .gap(px(8.))
            .items_center()
            .border_b_1()
            .border_color(style::hairline(cx))
            .child(
                div()
                    .text_size(px(size::LABEL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(style::label_text(cx))
                    .truncate()
                    .child(format!("Screens · {box_label}")),
            );
        if boxes_with_screens.len() > 1 {
            let mut tabs = h_flex().gap(px(4.)).items_center();
            for (bid, blabel, bcount, bpicked) in boxes_with_screens {
                let active = bid == node;
                let tab_label = match bpicked {
                    Some(v) => format!("{blabel} · {v}"),
                    None if bcount > 0 => format!("{blabel} ({bcount})"),
                    None => blabel.clone(),
                };
                let switch_id = bid.clone();
                tabs = tabs.child(
                    crate::a11y::A11yRole::a11y_role(
                        div()
                            .id(SharedString::from(format!("plan-screens-tab-{bid}")))
                            .flex_none()
                            .h(px(20.))
                            .px(px(7.))
                            .flex()
                            .items_center()
                            .whitespace_nowrap()
                            .rounded(px(5.))
                            .border_1()
                            .border_color(if active {
                                accent
                            } else {
                                style::button_edge(cx)
                            })
                            .bg(if active {
                                accent.opacity(0.16)
                            } else {
                                style::button_fill(cx)
                            })
                            .text_size(px(size::SMALL))
                            .font_weight(if active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if active {
                                accent
                            } else {
                                style::secondary_text(cx)
                            })
                            .cursor(CursorStyle::PointingHand)
                            .test_support(),
                        Role::Button,
                    )
                    .aria_label(SharedString::from(format!("Screens for {blabel}")))
                    .tip("Switch to this box's screens")
                    .child(tab_label)
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.screens_for = Some(switch_id.clone());
                            cx.notify();
                        },
                    )),
                );
            }
            header = header.child(tabs);
        }
        if let Some(v) = picked_variant {
            header = header.child(
                div()
                    .flex_none()
                    .h(px(20.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .rounded(px(5.))
                    .bg(green.opacity(0.16))
                    .text_size(px(size::SMALL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(green)
                    .child(format!("Picked: {v} · saved to box brief")),
            );
        } else if !board.state().approved() {
            header = header.child(
                div()
                    .flex_none()
                    .h(px(20.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .rounded(px(5.))
                    .bg(accent.opacity(0.14))
                    .text_size(px(size::SMALL))
                    .text_color(accent)
                    .child("Pick a take here, then click Approve plan"),
            );
        }
        header = header
            .child(div().flex_1())
            .child(
                bar_button_dyn(
                    format!("plan-screens-refresh-{node}"),
                    "Refresh",
                    "Reload screens for this box",
                    true,
                    cx,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.asking_screens_for = None;
                    this.load_frames_opts(None, false, cx);
                })),
            )
            .child(
                bar_button_dyn(
                    format!("plan-screens-close-{node}"),
                    "✕",
                    "Hide screens panel",
                    true,
                    cx,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.screens_for = None;
                    cx.notify();
                })),
            );

        let content: AnyElement = if !self.frames_loaded && mine.is_empty() {
            div()
                .p(px(16.))
                .text_size(px(size::DETAIL))
                .text_color(style::secondary_text(cx))
                .child("Loading screens…")
                .into_any_element()
        } else if mine.is_empty() {
            v_flex()
                .p(px(14.))
                .gap(px(10.))
                .child(
                    div()
                        .text_size(px(size::DETAIL))
                        .text_color(style::secondary_text(cx))
                        .child(format!(
                            "No screens for \"{box_label}\" yet. Generate 3 layout takes to compare side by side before approving, or mention this box in chat to attach a mockup you already picked."
                        )),
                )
                .child(
                    h_flex()
                        .gap(px(8.))
                        .items_center()
                        .child(
                            crate::a11y::A11yRole::a11y_role(
                                div()
                                    .id(SharedString::from(format!("plan-screens-generate-{node}")))
                                    .flex_none()
                                    .h(px(26.))
                                    .px(px(10.))
                                    .flex()
                                    .items_center()
                                    .whitespace_nowrap()
                                    .rounded(px(6.))
                                    .bg(accent)
                                    .text_size(px(size::DETAIL))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(gpui_kit::white())
                                    .when(asking, |d| d.opacity(0.5))
                                    .when(!asking, |d| {
                                        d.cursor(CursorStyle::PointingHand)
                                            .hover(|s| s.opacity(0.9))
                                    })
                                    .test_support(),
                                Role::Button,
                            )
                            .aria_label("Generate 3 takes")
                            .tip("Ask this thread's agent to draw 3 screen takes (A, B, C) for this box")
                            .child(if asking {
                                "Asking agent…"
                            } else {
                                "Generate 3 takes"
                            })
                            .when(!asking, |d| {
                                d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.ask_screens(&node_for_ask, cx)
                                }))
                            }),
                        )
                        .child(
                            bar_button_dyn(
                                format!("plan-screens-mention-{node}"),
                                "Mention box in chat",
                                "Put this box in the message box so you can describe the screen or reference a mockup",
                                true,
                                cx,
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.select(Hit::Box(node_for_mention.clone()), false, cx)
                            })),
                        ),
                )
                .into_any_element()
        } else {
            let mut strip = h_flex()
                .id("plan-screens-strip")
                .w_full()
                .overflow_x_scroll()
                .p(px(12.))
                .gap(px(12.))
                .items_start();
            for f in &mine {
                let picture = self.still(f, dark, cx);
                let edge = if f.picked {
                    green
                } else {
                    style::card_border(cx)
                };
                let (vw, vh) = rules::viewport_size(&f.viewport);
                let card_w: f32 = 232.;
                let img_h = (card_w * vh / vw).clamp(112., 165.);
                let words = format!(
                    "{} · {} · {} · v{}",
                    f.title, f.variant, f.viewport, f.version
                );
                let chip = rules::frame_chip(board, f);
                let chip_btn = chip.clone();
                let (pick_f, open_f) = (f.clone(), f.clone());
                let id = f.id.clone();
                let pending = self
                    .stills
                    .get(&(f.id.clone(), f.version, dark))
                    .is_some_and(Option::is_none);
                strip = strip.child(
                    v_flex()
                        .flex_none()
                        .w(px(card_w))
                        .gap(px(6.))
                        .p(px(8.))
                        .rounded(px(8.))
                        .bg(style::bubble_background(cx))
                        .border_1()
                        .border_color(if f.picked { green } else { style::hairline(cx) })
                        .child(
                            h_flex()
                                .gap(px(6.))
                                .items_center()
                                .child(
                                    div()
                                        .flex_none()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(4.))
                                        .bg(if f.picked {
                                            green.opacity(0.18)
                                        } else {
                                            accent.opacity(0.14)
                                        })
                                        .text_size(px(size::SMALL))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(if f.picked { green } else { accent })
                                        .child(f.variant.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(size::DETAIL))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(style::label_text(cx))
                                        .child(f.title.clone()),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .whitespace_nowrap()
                                        .text_size(px(size::SMALL))
                                        .text_color(style::quiet_text(cx))
                                        .child(format!("{} · v{}", f.viewport, f.version)),
                                )
                                .when(f.changed, |d| {
                                    d.child(
                                        div()
                                            .flex_none()
                                            .whitespace_nowrap()
                                            .text_size(px(size::SMALL))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(style::needs_you(cx))
                                            .child("Changed"),
                                    )
                                }),
                        )
                        .child(
                            crate::a11y::A11yRole::a11y_role(
                                div()
                                    .id(SharedString::from(format!("plan-screen-{id}")))
                                    .w_full()
                                    .h(px(img_h))
                                    .rounded(px(6.))
                                    .overflow_hidden()
                                    .border(px(if f.picked { 2. } else { 1. }))
                                    .border_color(edge)
                                    .bg(style::card_background(cx))
                                    .cursor(CursorStyle::PointingHand)
                                    .test_support(),
                                Role::Image,
                            )
                            .aria_label(SharedString::from(format!(
                                "Screen {words}{}",
                                if f.picked { ", picked" } else { "" }
                            )))
                            .tip("Click to put this screen in the message box")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.chip_text(chip.clone(), cx)
                            }))
                            .children(picture.map(|p| img(ImageSource::Image(p)).size_full()))
                            .when(pending, |d| {
                                d.child(
                                    div()
                                        .p(px(8.))
                                        .text_size(px(size::SMALL))
                                        .text_color(style::quiet_text(cx))
                                        .child(if f.route.is_some() {
                                            "Dev server page: click Open in browser"
                                        } else {
                                            "Drawing preview…"
                                        }),
                                )
                            }),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.))
                                .items_center()
                                .child(
                                    bar_button_dyn(
                                        format!("plan-pick-{id}"),
                                        if f.picked { "Picked ✓" } else { "Pick take" },
                                        if f.picked {
                                            "Selected take for this box (automatically included when you Send this box)"
                                        } else {
                                            "Save this take as the chosen design for this box and put its chip in chat"
                                        },
                                        !f.picked,
                                        cx,
                                    )
                                    .when(!f.picked, |d| {
                                        d.on_click(cx.listener(
                                            move |this, _: &ClickEvent, _, cx| {
                                                this.pick_frame(&pick_f, cx)
                                            },
                                        ))
                                    }),
                                )
                                .child(
                                    bar_button_dyn(
                                        format!("plan-chat-{id}"),
                                        "Discuss in chat",
                                        "Put this screen in the main thread message box so you can ask for changes",
                                        true,
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _, cx| {
                                            this.chip_text(chip_btn.clone(), cx)
                                        },
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
                                        .on_click(
                                            cx.listener(move |this, _: &ClickEvent, _, cx| {
                                                this.open_frame(&open_f, cx)
                                            }),
                                        ),
                                    )
                                }),
                        ),
                );
            }
            strip.into_any_element()
        };

        Some(
            v_flex()
                .id("plan-screens-drawer")
                .flex_none()
                .w_full()
                .bg(style::card_background(cx))
                .border_t_1()
                .border_color(style::hairline(cx))
                .child(header)
                .child(content)
                .into_any_element(),
        )
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
        self.load_frames_opts(Some(f.id.clone()), true, cx);
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

/// `bar_button` with an id made at run time (one per screen control).
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
            .flex_none()
            .h(px(22.))
            .px(px(8.))
            .flex()
            .items_center()
            .whitespace_nowrap()
            .rounded(px(5.))
            .border_1()
            .border_color(style::button_edge(cx))
            .bg(style::button_fill(cx))
            .text_size(px(size::SMALL))
            .text_color(style::label_text(cx))
            .when(!enabled, |d| d.opacity(0.45))
            .when(enabled, |d| {
                d.cursor(CursorStyle::PointingHand)
                    .hover(|s| s.bg(style::hover_fill(cx)))
            })
            .test_support(),
        Role::Button,
    )
    .aria_label(tip.clone())
    .tip(tip)
    .child(label)
}
