// The Browser pane: the pages a thread's agents are using through BrowserBear,
// in the agents' own browser (Helium or Chrome, in real windows behind Mike's) or in his
// own Chrome profiles. A port of the Swift app's AgentBrowserPanel.swift,
// BrowserBearPipe.swift and BrowserBear.swift (docs/protocol/panels.md, "The Browser
// panel"). Each row has a still of the page and Show window, which brings the
// real window forward. Nothing is streamed or drawn live here: Mike disliked
// the page drawn inside Threadbear (6 Nov 2026), so since 8 Nov he uses a page
// in its own window and the pane only says what's open. So there's no web
// view, and nothing here reaches a page: BrowserBear does what it's asked
// about pages its threads own, over its private socket.
//
// Also the sign-in handoff: a web link Mike clicks first asks BrowserBear to
// hand over an agent's tab already on that page (`handoff`), so the agent's
// waiting tab comes to him instead of a second one opening in his browser
// (the Qantas sign-in, 30 Oct). open_gate.rs calls it.
use crate::highlights::{icon, palette, tipped_as, with_scroll_bar, SidePane};
use crate::push_button::PushButton;
use crate::spinner::Spinner;
use crate::style::{self, px};
use crate::tips::Tip as _;
use futures::channel::oneshot;
use gpui_kit::assets::IconName;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex, Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use threadbear_client::runner::Client;
use threadbear_core::browserbear::{self as bear, AgentTab, PickState};
use threadbear_core::inbox::ThreadSummary;

pub(crate) use threadbear_core::browserbear::PANEL_ID as BROWSER;

/// How often tabs.json and the profiles' pages are looked at: a second's lag
/// is fine, it's for which pages a thread has and what the agent's doing.
const POLL: Duration = Duration::from_millis(750);
/// How often an active "Add from page" pick asks BrowserBear for its result.
const PICK_POLL: Duration = Duration::from_millis(250);
/// A row's still is asked for this often while it's on show; BrowserBear
/// decides how fresh it can be.
const STILL_EVERY: Duration = Duration::from_secs(3);
/// A button's request is given up on after this, and the button comes back.
const ANSWER_WITHIN: Duration = Duration::from_secs(45);
const STILL_WIDTH: f32 = 104.;
pub(crate) const PICK_BUTTON_TIP: &str =
    "Pick an element, region or text in this page's window to preview in your message";
pub(crate) const CANCEL_PICK_TIP: &str = "Stop picking on this page";

/// BrowserBear's folder, worked out once (`bear::home`): none for a scripted
/// app without `BROWSERBEAR_HOME`, so specs and dev-apps never reach the real one.
static HOME: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let env = std::env::var("BROWSERBEAR_HOME").ok();
    bear::home(
        env.as_deref(),
        Path::new(&crate::files::home_dir()),
        scripted(),
    )
});

/// Started by a script (the specs, an agent's dev-app): whatever the build,
/// as dev-app can run one made without the automation feature
/// (`installed::launch` reads the same marker).
fn scripted() -> bool {
    std::env::var_os("THREADBEAR_AUTOMATION").is_some()
}

/// BrowserBear's socket, when it's there.
fn socket() -> Option<PathBuf> {
    HOME.as_deref().map(bear::socket).filter(|s| s.exists())
}

#[cfg(unix)]
use crate::browserbear_start::{can_start, Starter};
#[cfg(not(unix))]
#[derive(Default)]
struct Starter;
#[cfg(not(unix))]
impl Starter {
    fn forget(&self) {}
}
#[cfg(not(unix))]
fn can_start(_: &Path) -> bool {
    false
}

/// tabs.json's bytes and what they say, when they differ from `last`. Only a
/// plain file, opened so a pipe put there can't hold the read, and only its
/// first `MAX_TABS_FILE` bytes: a bigger one is passed over.
fn read_tabs(
    file: &Path,
    last: Option<Arc<Vec<u8>>>,
) -> Option<(Arc<Vec<u8>>, Option<bear::AgentState>)> {
    use std::io::Read as _;
    let mut open = std::fs::OpenOptions::new();
    open.read(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut open, libc::O_NONBLOCK);
    let handle = open.open(file).ok()?;
    if !handle.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = vec![];
    handle
        .take(bear::MAX_TABS_FILE + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > bear::MAX_TABS_FILE || last.as_deref() == Some(&bytes) {
        return None;
    }
    let state = bear::agent_state(&bytes);
    Some((Arc::new(bytes), state))
}

// ---------------------------------------------------------------- the handoff

/// The longest a clicked link waits on BrowserBear, all told.
const HANDOFF_WITHIN: Duration = Duration::from_secs(4);

/// Asks BrowserBear to bring an agent's tab already on `url` to Mike (into his
/// window, in front). True only when it says it did; false straight away
/// without BrowserBear, and after 4 s at most when it doesn't answer: a tab an
/// agent opened is found at once, but ruling one out means listing Chrome's
/// tabs, which takes seconds while BrowserBear's extension wakes up (2 s was
/// too short, 30 Oct). Blocking: run it off the main thread.
pub(crate) fn handoff(url: &str) -> bool {
    if !bear::handoff_candidate(url) {
        return false;
    }
    let Some(path) = socket() else { return false };
    let line = bear::request_line(1, "handoff", json!({ "tab": url }), None);
    // On a thread of its own, so a connect that hangs (a full accept queue)
    // can't hold the link past the deadline. A connect can't be called off,
    // so a few may be under way at once (links clicked close together each
    // get theirs), and past that the link just opens in the browser: stuck
    // threads never pile up.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static ASKING: AtomicUsize = AtomicUsize::new(0);
    const AT_ONCE: usize = 4;
    if ASKING.fetch_add(1, Ordering::SeqCst) >= AT_ONCE {
        ASKING.fetch_sub(1, Ordering::SeqCst);
        return false;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let deadline = Instant::now() + HANDOFF_WITHIN;
    let started = std::thread::Builder::new()
        .name("browserbear-handoff".into())
        .spawn(move || {
            let _ = tx.send(one_request(&path, &line, deadline));
            ASKING.fetch_sub(1, Ordering::SeqCst);
        });
    if started.is_err() {
        ASKING.fetch_sub(1, Ordering::SeqCst);
        return false;
    }
    let Ok(Some(reply)) = rx.recv_timeout(HANDOFF_WITHIN) else {
        return false;
    };
    bear::handed_off(&reply, 1)
}

/// Whether a link should ask BrowserBear before it opens.
pub(crate) fn may_hand_off(url: &str) -> bool {
    bear::handoff_candidate(url) && socket().is_some()
}

/// One line out and one back on a connection of its own, by `deadline`.
#[cfg(unix)]
fn one_request(path: &Path, line: &[u8], deadline: Instant) -> Option<Vec<u8>> {
    use std::io::{Read as _, Write as _};
    let left = || {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
    };
    let mut stream = std::os::unix::net::UnixStream::connect(path).ok()?;
    stream.set_write_timeout(Some(left()?)).ok()?;
    stream.write_all(line).ok()?;
    let mut reply = vec![];
    let mut buffer = [0u8; 4096];
    while !reply.contains(&b'\n') {
        // What's left of the deadline each time, so a trickle can't stretch it.
        stream.set_read_timeout(Some(left()?)).ok()?;
        let n = stream.read(&mut buffer).ok()?;
        if n == 0 || reply.len() > 1 << 20 {
            return None;
        }
        reply.extend_from_slice(&buffer[..n]);
    }
    reply.truncate(
        reply
            .iter()
            .position(|b| *b == b'\n')
            .unwrap_or(reply.len()),
    );
    Some(reply)
}

#[cfg(not(unix))]
fn one_request(_path: &Path, _line: &[u8], _deadline: Instant) -> Option<Vec<u8>> {
    None
}

// ---------------------------------------------------------------- the pipe

type Answer = Result<Value, String>;

/// Requests waiting to be written at most: past it the other side isn't
/// reading, and a request is refused rather than queued.
const QUEUE: usize = 64;

/// BrowserBear's private Unix socket, kept open while the app runs, with
/// replies matched to requests by id (`BrowserBearPipe`). A thread connects
/// and writes, another reads, so nothing here ever waits on the main thread.
struct Pipe {
    writer: std::sync::mpsc::SyncSender<Vec<u8>>,
    waiting: Arc<Mutex<HashMap<u64, oneshot::Sender<Answer>>>>,
    /// Why it closed, once it has.
    dead: Arc<Mutex<Option<String>>>,
    next: u64,
    /// The connection, once the writer has made it, and whether the pipe
    /// has been dropped: one lock for both, so Drop and a connect finishing
    /// can't miss each other. Shutting it ends both threads whatever they're
    /// waiting on.
    #[cfg(unix)]
    stream: Arc<Mutex<Slot>>,
    /// Still connecting: no second pipe is started until this one's connect
    /// returns, as a connect can't be called off.
    connecting: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(unix)]
#[derive(Default)]
struct Slot {
    stream: Option<std::os::unix::net::UnixStream>,
    closed: bool,
}

impl Pipe {
    /// Starts connecting; requests queue meanwhile. A failure shows as `dead`.
    #[cfg(unix)]
    fn connect(path: PathBuf, home: PathBuf, starter: Arc<Starter>, start: bool) -> Option<Pipe> {
        use std::io::{Read as _, Write as _};
        let waiting: Arc<Mutex<HashMap<u64, oneshot::Sender<Answer>>>> = Arc::default();
        let dead: Arc<Mutex<Option<String>>> = Arc::default();
        let stream: Arc<Mutex<Slot>> = Arc::default();
        let connecting = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let still_connecting = connecting.clone();
        let die = {
            let (waiting, dead) = (waiting.clone(), dead.clone());
            move |why: String| {
                let mut d = dead.lock().unwrap_or_else(|e| e.into_inner());
                if d.is_none() {
                    *d = Some(why.clone());
                }
                for (_, reply) in waiting.lock().unwrap_or_else(|e| e.into_inner()).drain() {
                    let _ = reply.send(Err(why.clone()));
                }
            }
        };
        let (writer, lines) = std::sync::mpsc::sync_channel::<Vec<u8>>(QUEUE);
        let (answers, shared, read_dead) = (waiting.clone(), stream.clone(), die.clone());
        std::thread::Builder::new()
            .name("browserbear-write".into())
            .spawn(move || {
                let gone = || shared.lock().unwrap_or_else(|e| e.into_inner()).closed;
                let connected = starter.connect(&home, &path, start, gone).and_then(|s| {
                    let copy = || s.try_clone().map_err(|e| e.to_string());
                    Ok((copy()?, copy()?, s))
                });
                still_connecting.store(false, std::sync::atomic::Ordering::SeqCst);
                let (mut writing, mut reader, kept) = match connected {
                    Ok(c) => c,
                    Err(why) => return die(why),
                };
                {
                    let mut slot = shared.lock().unwrap_or_else(|e| e.into_inner());
                    // Dropped while connecting: nothing queued goes.
                    if slot.closed {
                        let _ = kept.shutdown(std::net::Shutdown::Both);
                        return;
                    }
                    slot.stream = Some(kept);
                }
                let read = std::thread::Builder::new()
                    .name("browserbear-read".into())
                    .spawn(move || {
                        let die = read_dead;
                        let mut lines = bear::Lines::default();
                        let mut buffer = vec![0u8; 1 << 20];
                        loop {
                            let n =
                                match reader.read(&mut buffer) {
                                    Ok(0) => return die(
                                        "BrowserBear disconnected. Reopen the panel to reconnect."
                                            .into(),
                                    ),
                                    Ok(n) => n,
                                    Err(e) => return die(e.to_string()),
                                };
                            let got = match lines.push(&buffer[..n]) {
                                Ok(got) => got,
                                Err(why) => return die(why),
                            };
                            for line in got {
                                match bear::read_line(&line) {
                                    Some(bear::Line::Reply { id, answer }) => {
                                        if let Some(reply) = answers
                                            .lock()
                                            .unwrap_or_else(|e| e.into_inner())
                                            .remove(&id)
                                        {
                                            let _ = reply.send(answer);
                                        }
                                    }
                                    // Nothing the pane uses comes unasked.
                                    Some(bear::Line::Event(_)) => {}
                                    None => {
                                        return die(
                                            "BrowserBear sent an unreadable response.".into()
                                        )
                                    }
                                }
                            }
                        }
                    });
                if read.is_err() {
                    return die("Couldn't start reading BrowserBear's answers.".into());
                }
                for line in lines {
                    // A request queued before the pipe was dropped (one that
                    // timed out, say) is never sent after.
                    if shared.lock().unwrap_or_else(|e| e.into_inner()).closed {
                        break;
                    }
                    if let Err(e) = writing.write_all(&line) {
                        return die(e.to_string());
                    }
                }
                // The pipe's gone: end the reader too, if Drop didn't.
                if let Some(stream) = shared
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stream
                    .take()
                {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            })
            .ok()?;
        Some(Pipe {
            writer,
            waiting,
            dead,
            next: 1,
            stream,
            connecting,
        })
    }

    #[cfg(not(unix))]
    fn connect(_: PathBuf, _: PathBuf, _: Arc<Starter>, _: bool) -> Option<Pipe> {
        None
    }

    fn dead(&self) -> Option<String> {
        self.dead.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Sends one request; its answer comes on the receiver.
    fn request(
        &mut self,
        cmd: &str,
        args: Value,
        thread: Option<&str>,
    ) -> (u64, oneshot::Receiver<Answer>) {
        let (tx, rx) = oneshot::channel();
        let id = self.next;
        self.next += 1;
        if let Some(why) = self.dead() {
            let _ = tx.send(Err(why));
            return (id, rx);
        }
        self.waiting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        if self
            .writer
            .try_send(bear::request_line(id, cmd, args, thread))
            .is_err()
        {
            if let Some(tx) = self
                .waiting
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id)
            {
                let _ = tx.send(Err(
                    "BrowserBear isn't keeping up. Try again in a moment.".into()
                ));
            }
        }
        (id, rx)
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        // Ends the reader and a writer stuck writing; one still connecting
        // sees it's alone and stops.
        #[cfg(unix)]
        {
            let mut slot = self.stream.lock().unwrap_or_else(|e| e.into_inner());
            slot.closed = true;
            if let Some(stream) = slot.stream.take() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        }
    }
}

/// An answer, or why there isn't one within `ANSWER_WITHIN`.
/// True with it when the time ran out.
async fn answer(rx: oneshot::Receiver<Answer>, cx: &AsyncApp) -> (Answer, bool) {
    use futures::future::{select, Either};
    let timer = cx.background_executor().timer(ANSWER_WITHIN);
    match select(rx, Box::pin(timer)).await {
        Either::Left((Ok(answer), _)) => (answer, false),
        Either::Left((Err(_), _)) => (
            Err("BrowserBear disconnected. Reopen the panel to reconnect.".into()),
            false,
        ),
        Either::Right(_) => (Err("BrowserBear didn't answer in time.".into()), true),
    }
}

// ---------------------------------------------------------------- the model

/// A page's still, as the pane last got it.
#[derive(Clone)]
pub(crate) struct Still {
    pub(crate) image: Arc<Image>,
    /// When BrowserBear took it, ms since 1970.
    pub(crate) at: Option<f64>,
}

/// One active "Add from page" pick session, pinned to the thread that started
/// it so later thread switches cannot redirect the resulting preview.
#[derive(Clone)]
pub(crate) struct ActivePick {
    pub(crate) capture_id: String,
    pub(crate) thread_id: String,
    pub(crate) tab_key: String,
    pub(crate) tab_title: String,
    pub(crate) mode: String,
    /// Bring the app back to the front on its thread when the pick ends
    /// (Settings > General > Browser).
    pub(crate) come_back: bool,
}

/// Reads the agent host's tabs.json and asks BrowserBear for the pages in
/// Mike's own profiles (`AgentBrowserModel`). One for the app.
pub(crate) struct AgentBrowser {
    pub(crate) tabs: Vec<AgentTab>,
    agent_tabs: Vec<AgentTab>,
    profile_tabs: Vec<AgentTab>,
    port: i64,
    socket_there: bool,
    /// Its service can be started, so the pane shows with no socket yet.
    startable: bool,
    /// A Browser pane was on show at the last read. Kept, as `showing` reads
    /// the app's own entity and a request can come from inside its update.
    on_show: bool,
    starter: Arc<Starter>,
    pub(crate) browser_name: String,
    pub(crate) error: Option<String>,
    request_error: Option<String>,
    /// The one active "Add from page" pick on this Mac, if any.
    pub(crate) active_pick: Option<ActivePick>,
    /// User-facing explanation when a pick fails, moves away, or is refused.
    pub(crate) pick_note: Option<String>,
    pick_serial: u64,
    pick_task: Option<Task<()>>,
    /// Buttons waiting for BrowserBear's answer, by `pending_key`. It can
    /// take a while, as the browser has to do the thing.
    pub(crate) pending: HashSet<String>,
    /// The request each key waits on, so an old request's timeout can't
    /// clear a newer one's spinner.
    pending_requests: HashMap<String, u64>,
    next_request: u64,
    pipe: Option<Pipe>,
    /// Goes up with each new connection, so a late timeout drops only its own.
    pipe_serial: u64,
    /// The last dropped pipe's connect, if it may still be stuck.
    stuck_connect: Option<Arc<std::sync::atomic::AtomicBool>>,
    polling: bool,
    /// tabs.json as last read.
    last: Option<Arc<Vec<u8>>>,
    pub(crate) stills: HashMap<String, Still>,
    /// Goes up when the pages or whether BrowserBear is here change: what the
    /// window's rail draws, so a new still doesn't redraw the whole window.
    pub(crate) generation: u64,
    stills_asked: HashMap<String, Instant>,
    /// The thread whose Browser pane is on show, if one is: its rows' stills
    /// are asked for as they come due.
    showing: Option<Box<dyn Fn(&App) -> Option<String>>>,
    come_back: Option<Box<dyn Fn(String, &mut App)>>,
    _poll: Task<()>,
}

/// The app's one AgentBrowser.
pub(crate) struct Browsers(pub(crate) Entity<AgentBrowser>);
impl Global for Browsers {}

pub(crate) fn install(cx: &mut App) -> Entity<AgentBrowser> {
    if let Some(b) = cx.try_global::<Browsers>() {
        return b.0.clone();
    }
    let entity = cx.new(AgentBrowser::new);
    cx.set_global(Browsers(entity.clone()));
    entity
}

pub(crate) fn model(cx: &App) -> Option<Entity<AgentBrowser>> {
    cx.try_global::<Browsers>().map(|b| b.0.clone())
}

/// Where a page's window goes beside this one, if there's room on its screen.
fn place_beside(window: &Window, cx: &App) -> Option<bear::Place> {
    let place = |b: Bounds<Pixels>| bear::Place {
        left: f64::from(f32::from(b.origin.x)),
        top: f64::from(f32::from(b.origin.y)),
        width: f64::from(f32::from(b.size.width)),
        height: f64::from(f32::from(b.size.height)),
    };
    if window.is_fullscreen() {
        return None;
    }
    let screen = window.display(cx)?.bounds();
    bear::beside(place(window.bounds()), place(screen))
}

pub(crate) const OPENING_KEY: &str = "open";

pub(crate) fn pending_key(cmd: &str, tab: &AgentTab) -> String {
    format!("{cmd}/{}", tab.id())
}

impl AgentBrowser {
    fn new(cx: &mut Context<Self>) -> Self {
        let poll = cx.spawn(async move |this, cx| loop {
            let Ok(last) = this.update(cx, |b, cx| {
                b.read(cx);
                b.last.clone()
            }) else {
                break;
            };
            // tabs.json is read and parsed off the main thread, only when
            // its bytes changed.
            if let Some(home) = HOME.clone() {
                let read = cx
                    .background_executor()
                    .spawn(async move { read_tabs(&bear::tabs_file(&home), last) })
                    .await;
                if let Some((bytes, state)) = read {
                    if this
                        .update(cx, |b, cx| b.took_tabs(bytes, state, cx))
                        .is_err()
                    {
                        break;
                    }
                }
            }
            cx.background_executor().timer(POLL).await;
        });
        AgentBrowser {
            tabs: vec![],
            agent_tabs: vec![],
            profile_tabs: vec![],
            port: 0,
            socket_there: false,
            startable: false,
            on_show: false,
            starter: Arc::default(),
            // Until its host says which: Helium, or Google Chrome without it.
            browser_name: "the agents' browser".into(),
            error: None,
            request_error: None,
            active_pick: None,
            pick_note: None,
            pick_serial: 0,
            pick_task: None,
            pending: HashSet::new(),
            pending_requests: HashMap::new(),
            next_request: 0,
            pipe: None,
            pipe_serial: 0,
            stuck_connect: None,
            polling: false,
            last: None,
            stills: HashMap::new(),
            generation: 0,
            stills_asked: HashMap::new(),
            showing: None,
            come_back: None,
            _poll: poll,
        }
    }

    /// BrowserBear is on this Mac: the rail's Browser button shows.
    pub(crate) fn available(&self) -> bool {
        self.port != 0 || self.socket_there || self.startable
    }

    pub(crate) fn tabs_for(&self, thread: &str) -> Vec<AgentTab> {
        bear::for_thread(&self.tabs, thread)
    }

    /// How to bring the app's window forward on a thread, after a pick.
    pub(crate) fn on_come_back(&mut self, back: impl Fn(String, &mut App) + 'static) {
        self.come_back = Some(Box::new(back));
    }

    /// The pick that just ended was for `thread`: back to it, if asked.
    fn came_back(&self, pick: Option<ActivePick>, cx: &mut Context<Self>) {
        if let (Some(pick), Some(back)) = (pick.filter(|p| p.come_back), &self.come_back) {
            back(pick.thread_id, cx);
        }
    }

    /// How to tell which thread's pane is on show (the window's dock).
    pub(crate) fn watch_pane(&mut self, showing: impl Fn(&App) -> Option<String> + 'static) {
        self.showing = Some(Box::new(showing));
    }

    fn read(&mut self, cx: &mut Context<Self>) {
        let Some(home) = HOME.clone() else { return };
        let shown = self.showing.as_ref().and_then(|f| f(cx));
        self.on_show = shown.is_some();
        if let Some(thread) = shown {
            self.want_stills(&thread, cx);
        }
        let there = bear::socket(&home).exists();
        let startable = can_start(&home);
        if (there, startable) != (self.socket_there, self.startable) {
            (self.socket_there, self.startable) = (there, startable);
            self.generation += 1;
            cx.notify();
        }
        self.notice_closed(cx);
        self.poll_profiles(cx);
    }

    /// What `read_tabs` found new in tabs.json.
    fn took_tabs(
        &mut self,
        bytes: Arc<Vec<u8>>,
        state: Option<bear::AgentState>,
        cx: &mut Context<Self>,
    ) {
        self.last = Some(bytes);
        let Some(state) = state else { return };
        self.agent_tabs = state.tabs;
        if let Some(browser) = state.browser.filter(|b| !b.is_empty()) {
            self.browser_name = browser;
        }
        if state.port != self.port {
            self.port = state.port;
            self.generation += 1;
            cx.notify();
        }
        self.merge(cx);
    }

    fn merge(&mut self, cx: &mut Context<Self>) {
        let tabs = bear::merge(&self.agent_tabs, &self.profile_tabs);
        // Only a change is told: the window redraws for each.
        if tabs != self.tabs {
            self.tabs = tabs;
            let live: HashSet<String> = self.tabs.iter().map(AgentTab::id).collect();
            self.stills.retain(|id, _| live.contains(id));
            self.stills_asked.retain(|id, _| live.contains(id));
            self.generation += 1;
            cx.notify();
        }
    }

    fn set_error(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        if error != self.error {
            self.error = error;
            cx.notify();
        }
    }

    /// A connection that closed is said, and the next request opens another.
    fn notice_closed(&mut self, cx: &mut Context<Self>) {
        let Some(why) = self.pipe.as_ref().and_then(Pipe::dead) else {
            return;
        };
        self.pipe = None;
        self.set_error(Some(why), cx);
        if self.active_pick.take().is_some() {
            self.pick_task = None;
            self.pick_serial += 1;
            self.pick_note = Some("BrowserBear disconnected. Nothing was added.".into());
        }
        self.polling = false;
        self.profile_tabs.clear();
        self.merge(cx);
    }

    /// Try again, under an error: starts BrowserBear now if it's down.
    pub(crate) fn retry(&mut self, cx: &mut Context<Self>) {
        self.starter.forget();
        self.on_show = true;
        self.request_error = None;
        self.set_error(None, cx);
        if self.pipe.as_ref().is_some_and(|p| p.dead().is_some()) {
            self.pipe = None;
            self.polling = false;
        }
        self.poll_profiles(cx);
    }

    /// `start`: something on show or asked for needs it, so a service that's
    /// down is started. A pane nobody has open never starts one.
    fn connection(&mut self, start: bool) -> Option<&mut Pipe> {
        if self.pipe.is_none() {
            // One dropped while its connect was stuck: wait for that to return.
            if self
                .stuck_connect
                .as_ref()
                .is_some_and(|c| c.load(std::sync::atomic::Ordering::SeqCst))
            {
                return None;
            }
            let home = HOME.clone()?;
            let path = bear::socket(&home);
            if !self.socket_there && !start {
                return None;
            }
            self.pipe = Pipe::connect(path, home, self.starter.clone(), start);
            self.pipe_serial += 1;
        }
        self.pipe.as_mut()
    }

    /// Sends a request and hands its answer to `done` on the main thread.
    fn send(
        &mut self,
        cmd: &str,
        args: Value,
        thread: Option<&str>,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, Answer, &mut Context<Self>) + 'static,
    ) -> bool {
        let polled = matches!(cmd, "panelState" | "panelStill" | "panelPickLook");
        let start = !polled || self.on_show;
        let Some(pipe) = self.connection(start) else {
            return false;
        };
        let (_, rx) = pipe.request(cmd, args, thread);
        let serial = self.pipe_serial;
        cx.spawn(async move |this, cx| {
            let (got, timed_out) = answer(rx, cx).await;
            this.update(cx, |b, cx| {
                // BrowserBear stopped answering on this connection: drop it,
                // with whatever is still queued on it, so the next request
                // starts afresh rather than behind a stalled one.
                if timed_out && b.pipe_serial == serial {
                    if let Some(pipe) = b.pipe.take() {
                        b.stuck_connect = Some(pipe.connecting.clone());
                    }
                    b.polling = false;
                }
                done(b, got, cx);
            })
            .ok();
        })
        .detach();
        true
    }

    fn poll_profiles(&mut self, cx: &mut Context<Self>) {
        if self.polling {
            return;
        }
        self.polling = self.send("panelState", json!({}), None, cx, |b, got, cx| {
            b.polling = false;
            match got {
                Ok(result) => match serde_json::from_value::<bear::PanelState>(result) {
                    Ok(state) => {
                        b.set_error(b.request_error.clone(), cx);
                        b.profile_tabs = state
                            .tabs
                            .into_iter()
                            .filter(|t| t.bridge == Some(true))
                            .collect();
                        b.merge(cx);
                    }
                    Err(_) => b.set_error(
                        Some("BrowserBear sent an unreadable list of pages.".into()),
                        cx,
                    ),
                },
                Err(why) => b.set_error(Some(why), cx),
            }
        });
    }

    /// A button's request: `key` is what its spinner watches, in `pending`,
    /// until the answer comes.
    fn action(
        &mut self,
        cmd: &str,
        args: Value,
        thread: Option<&str>,
        key: String,
        cx: &mut Context<Self>,
    ) {
        self.request_error = None;
        self.error = None;
        self.next_request += 1;
        let request = self.next_request;
        self.pending.insert(key.clone());
        self.pending_requests.insert(key.clone(), request);
        cx.notify();
        let sent = self.send(cmd, args, thread, cx, move |b, got, cx| {
            if let Err(why) = got {
                b.request_error = Some(why.clone());
                b.error = Some(why);
            }
            b.answered(&key, request);
            cx.notify();
        });
        if !sent {
            self.error = Some("BrowserBear isn't running.".into());
            let key = self
                .pending_requests
                .iter()
                .find(|(_, r)| **r == request)
                .map(|(k, _)| k.clone());
            if let Some(key) = key {
                self.answered(&key, request);
            }
        }
    }

    fn answered(&mut self, key: &str, request: u64) {
        if self.pending_requests.get(key) != Some(&request) {
            return;
        }
        self.pending_requests.remove(key);
        self.pending.remove(key);
    }

    /// Done: tells the agent waiting on `handoff --wait` that Mike's finished.
    pub(crate) fn done(&mut self, tab: &AgentTab, cx: &mut Context<Self>) {
        self.action(
            "markDone",
            json!({ "tab": tab.tab_id }),
            None,
            pending_key("markDone", tab),
            cx,
        );
    }

    /// Show window: brings the page's real window to the front.
    /// `place`: where it should go, beside the app's window (`place_beside`).
    pub(crate) fn show(
        &mut self,
        tab: &AgentTab,
        thread: &str,
        place: Option<bear::Place>,
        cx: &mut Context<Self>,
    ) {
        let mut args = json!({ "tab": tab.tab_id, "profile": tab.profile_id() });
        if let Some(place) = place {
            args["place"] = place.json();
        }
        self.action(
            "panelShow",
            args,
            Some(thread),
            pending_key("panelShow", tab),
            cx,
        );
    }

    /// A new page in the agents' browser, behind Mike's windows.
    pub(crate) fn open(&mut self, url: &str, thread: &str, cx: &mut Context<Self>) {
        self.action(
            "open",
            json!({ "profile": "agent", "url": url }),
            Some(thread),
            OPENING_KEY.to_owned(),
            cx,
        );
    }

    /// Starts an "Add from page" pick session in the real browser window,
    /// pinning `thread` as the only destination for its capture.
    pub(crate) fn start_pick(
        &mut self,
        tab: &AgentTab,
        thread: &str,
        mode: &str,
        // Where its window goes, and whether the app comes back after.
        (place, come_back): (Option<bear::Place>, bool),
        client: Arc<Client>,
        cx: &mut Context<Self>,
    ) {
        if let Some(prev) = self.active_pick.take() {
            self.pick_task = None;
            let _ = self.send(
                "panelPickCancel",
                json!({ "captureId": prev.capture_id }),
                Some(&prev.thread_id),
                cx,
                |_, _, _| {},
            );
        }
        self.pick_serial += 1;
        let serial = self.pick_serial;
        let capture_id = format!("pick-{}-{serial}", crate::now_ms());
        let pinned_thread = thread.to_owned();
        let tab_key = tab.id();
        let key = pending_key("panelPick", tab);
        self.next_request += 1;
        let request = self.next_request;
        self.pending.insert(key.clone());
        self.pending_requests.insert(key.clone(), request);
        self.pick_note = None;
        self.request_error = None;
        self.error = None;
        self.active_pick = Some(ActivePick {
            capture_id: capture_id.clone(),
            thread_id: pinned_thread.clone(),
            tab_key,
            tab_title: tab.shown_title(),
            mode: mode.to_owned(),
            come_back,
        });
        cx.notify();
        let mut args = json!({
            "tab": tab.tab_id,
            "profile": tab.profile_id(),
            "captureId": capture_id,
            "mode": mode,
        });
        if let Some(place) = place {
            args["place"] = place.json();
        }
        let key_cb = key.clone();
        let sent = self.send("panelPick", args, Some(thread), cx, move |b, got, cx| {
            b.answered(&key_cb, request);
            if b.pick_serial != serial {
                cx.notify();
                return;
            }
            match got {
                Ok(result) => match bear::parse_pick_reply(&result, &capture_id, &pinned_thread) {
                    Some(reply) if reply.state == PickState::Done => {
                        b.active_pick = None;
                        b.pick_task = None;
                        if let Some(capture) = reply.capture {
                            crate::composer_captures::insert(capture, client, cx);
                        }
                    }
                    Some(reply) if reply.state == PickState::Picking => {
                        if let Some(active) = b.active_pick.as_mut() {
                            if let Some(m) = reply.mode {
                                active.mode = m;
                            }
                        }
                        b.spawn_pick_poll(serial, capture_id, pinned_thread, client, cx);
                    }
                    Some(reply) => {
                        b.active_pick = None;
                        b.pick_task = None;
                        if reply.state != PickState::Cancelled {
                            b.pick_note = reply.note;
                        }
                    }
                    None => {
                        b.active_pick = None;
                        b.pick_task = None;
                        b.pick_note = Some("BrowserBear sent an unexpected pick reply.".into());
                    }
                },
                Err(why) => {
                    b.active_pick = None;
                    b.pick_task = None;
                    b.pick_note = Some(why);
                }
            }
            cx.notify();
        });
        if !sent {
            self.answered(&key, request);
            self.active_pick = None;
            self.pick_note = Some("BrowserBear isn't running.".into());
            cx.notify();
        }
    }

    fn spawn_pick_poll(
        &mut self,
        serial: u64,
        capture_id: String,
        pinned_thread: String,
        client: Arc<Client>,
        cx: &mut Context<Self>,
    ) {
        let task = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(PICK_POLL).await;
            let req = this.update(cx, |b, _| {
                if b.pick_serial != serial || b.active_pick.is_none() {
                    return None;
                }
                let pipe = b.connection(false)?;
                let (_, rx) = pipe.request(
                    "panelPickLook",
                    json!({ "captureId": capture_id }),
                    Some(&pinned_thread),
                );
                Some((rx, b.pipe_serial))
            });
            let Ok(Some((rx, pipe_serial))) = req else {
                break;
            };
            let (got, timed_out) = answer(rx, cx).await;
            let keep_going = this.update(cx, |b, cx| {
                if timed_out && b.pipe_serial == pipe_serial {
                    if let Some(pipe) = b.pipe.take() {
                        b.stuck_connect = Some(pipe.connecting.clone());
                    }
                    b.polling = false;
                }
                if b.pick_serial != serial || b.active_pick.is_none() {
                    return false;
                }
                match got {
                    Ok(result) => {
                        let Some(reply) =
                            bear::parse_pick_reply(&result, &capture_id, &pinned_thread)
                        else {
                            // Ignore mismatched captureId or thread replies.
                            return true;
                        };
                        match reply.state {
                            PickState::Picking => {
                                if let Some(active) = b.active_pick.as_mut() {
                                    if let Some(m) = reply.mode {
                                        if active.mode != m {
                                            active.mode = m;
                                            cx.notify();
                                        }
                                    }
                                }
                                true
                            }
                            PickState::Done => {
                                let pick = b.active_pick.take();
                                b.pick_task = None;
                                if let Some(capture) = reply.capture {
                                    crate::composer_captures::insert(capture, client.clone(), cx);
                                }
                                b.came_back(pick, cx);
                                cx.notify();
                                false
                            }
                            PickState::Cancelled => {
                                // Escape in the page: back here too.
                                let pick = b.active_pick.take();
                                b.came_back(pick, cx);
                                b.pick_task = None;
                                b.pick_note = None;
                                cx.notify();
                                false
                            }
                            PickState::Gone
                            | PickState::Replaced
                            | PickState::Abandoned
                            | PickState::Timeout
                            | PickState::Failed
                            | PickState::Unknown => {
                                b.active_pick = None;
                                b.pick_task = None;
                                b.pick_note = reply.note.or_else(|| {
                                    Some("The pick session ended. Nothing was added.".into())
                                });
                                cx.notify();
                                false
                            }
                        }
                    }
                    Err(why) => {
                        b.active_pick = None;
                        b.pick_task = None;
                        b.pick_note = Some(why);
                        cx.notify();
                        false
                    }
                }
            });
            if !matches!(keep_going, Ok(true)) {
                break;
            }
        });
        self.pick_task = Some(task);
    }

    /// Cancels the active pick session and tells BrowserBear to clean up.
    pub(crate) fn cancel_pick(&mut self, cx: &mut Context<Self>) {
        self.pick_serial += 1;
        self.pick_task = None;
        self.pick_note = None;
        if let Some(pick) = self.active_pick.take() {
            let _ = self.send(
                "panelPickCancel",
                json!({ "captureId": pick.capture_id }),
                Some(&pick.thread_id),
                cx,
                |_, _, _| {},
            );
        }
        cx.notify();
    }

    pub(crate) fn dismiss_pick_note(&mut self, cx: &mut Context<Self>) {
        if self.pick_note.take().is_some() {
            cx.notify();
        }
    }

    /// Asks for the stills of a thread's pages that are due, while its pane
    /// is on show. None for a background tab Chrome isn't drawing, or one
    /// handed to Mike in his own Chrome, which BrowserBear leaves alone.
    fn want_stills(&mut self, thread: &str, cx: &mut Context<Self>) {
        let now = Instant::now();
        let due: Vec<AgentTab> = self
            .tabs_for(thread)
            .into_iter()
            .filter(|t| {
                self.stills_asked
                    .get(&t.id())
                    .is_none_or(|at| now.duration_since(*at) >= STILL_EVERY)
            })
            .collect();
        for tab in due {
            let id = tab.id();
            self.stills_asked.insert(id.clone(), now);
            let args = json!({ "tab": tab.tab_id, "profile": tab.profile_id(), "width": (STILL_WIDTH * 2.) as i64 });
            self.send("panelStill", args, Some(thread), cx, move |b, got, cx| {
                if let Some((bytes, at)) = got.ok().as_ref().and_then(bear::still) {
                    b.stills.insert(
                        id,
                        Still {
                            image: Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)),
                            at,
                        },
                    );
                    cx.notify();
                }
            });
        }
    }
}

// ---------------------------------------------------------------- the pane

/// The address bar's state, made the first time the pane draws.
#[derive(Default)]
pub(crate) struct BrowserPaneState {
    address: Option<Entity<InputState>>,
    /// The list of pages' scroll, for its scroll bar.
    scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

impl SidePane {
    fn browser_address(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(input) = &self.browser.address {
            return input.clone();
        }
        let input = cx.new(|cx| InputState::new(window, cx));
        let sub = cx.subscribe_in(
            &input,
            window,
            |this, input, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    this.browser_go(input, window, cx);
                }
            },
        );
        self.browser.address = Some(input.clone());
        self.browser._subs.push(sub);
        input
    }

    fn browser_go(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(url) = bear::address(&input.read(cx).value()) else {
            return;
        };
        let (Some(thread), Some(model)) = (self.thread(cx), model(cx)) else {
            return;
        };
        model.update(cx, |b, cx| b.open(&url, &thread.id, cx));
        input.update(cx, |s, cx| s.set_value("", window, cx));
        window.blur(cx);
    }

    /// The Browser pane (`AgentBrowserPanel`): the address bar, then the
    /// thread's pages, Mike's turn first.
    pub(crate) fn render_browser_panel(
        &mut self,
        thread: &ThreadSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx);
        let Some(model) = model(cx) else {
            return crate::files::pane_note("BrowserBear isn't running.", p).into_any_element();
        };
        let b = model.read(cx);
        let tabs = b.tabs_for(&thread.id);
        let (available, opening, browser_name, error, active_pick, pick_note) = (
            b.available(),
            b.pending.contains(OPENING_KEY),
            b.browser_name.clone(),
            b.error.clone(),
            b.active_pick.clone(),
            b.pick_note.clone(),
        );
        let input = self.browser_address(window, cx);
        input.update(cx, |s, cx| {
            s.set_placeholder(format!("Open a page in {browser_name}"), window, cx)
        });
        let field = div()
            .flex_1()
            .min_w_0()
            .px(px(8.))
            .py(px(2.))
            .rounded(px(6.))
            .bg(p.hover)
            .child(
                Input::new(&input)
                    .appearance(false)
                    .small()
                    .disabled(!available || opening)
                    .text_size(px(style::size::LABEL)),
            );
        let bar = h_flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .px(px(10.))
            .py(px(6.))
            .child(field)
            .when(opening, |s| {
                s.child(Spinner::new().color(p.secondary).with_size(px(11.)))
            });
        let mut pane = v_flex().size_full().child(bar);
        if let Some(pick) = active_pick {
            let cancel_model = model.clone();
            let note = if pick.thread_id == thread.id {
                format!(
                    "Picking on {} ({})… Press Esc in the page or Cancel.",
                    pick.tab_title, pick.mode
                )
            } else {
                format!(
                    "Picking on {} for another thread… Press Esc or Cancel.",
                    pick.tab_title
                )
            };
            pane = pane.child(
                h_flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .mx(px(10.))
                    .mb(px(6.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(style::chip_fill(cx))
                    .border(gpui_kit::px(0.5))
                    .border_color(style::accent(cx).opacity(0.5))
                    .child(Spinner::new().color(style::accent(cx)).with_size(px(11.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(style::size::LABEL))
                            .text_color(p.fg)
                            .child(note),
                    )
                    .child(
                        PushButton::new("browser-pick-cancel-bar", "Cancel", CANCEL_PICK_TIP)
                            .small()
                            .render(cx, move |_, _, cx| {
                                cancel_model.update(cx, |b, cx| b.cancel_pick(cx))
                            }),
                    ),
            );
        }
        if let Some(note) = pick_note {
            let dismiss_model = model.clone();
            pane = pane.child(
                h_flex()
                    .flex_none()
                    .items_start()
                    .gap(px(6.))
                    .mx(px(10.))
                    .mb(px(6.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(7.))
                    .bg(style::chip_fill(cx))
                    .border(gpui_kit::px(0.5))
                    .border_color(style::hairline(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(style::size::LABEL))
                            .text_color(p.secondary)
                            .child(note),
                    )
                    .child(
                        div()
                            .id("browser-pick-note-dismiss")
                            .test_support()
                            .role(Role::Button)
                            .aria_label("Dismiss note")
                            .cursor_pointer()
                            .flex_none()
                            .tip("Dismiss note")
                            .on_click(move |_, _, cx| {
                                dismiss_model.update(cx, |b, cx| b.dismiss_pick_note(cx));
                            })
                            .child(
                                Icon::from(IconName::X)
                                    .with_size(px(10.))
                                    .text_color(p.secondary),
                            ),
                    ),
            );
        }
        if error.is_some() {
            let retry_model = model.clone();
            pane = pane.child(
                h_flex().flex_none().px(px(10.)).pb(px(6.)).child(
                    PushButton::new(
                        "browser-retry",
                        "Try again",
                        "Connect to BrowserBear again, starting it if it isn't running",
                    )
                    .small()
                    .render(cx, move |_, _, cx| {
                        retry_model.update(cx, |b, cx| b.retry(cx))
                    }),
                ),
            );
        }
        if tabs.is_empty() {
            return pane
                .child(crate::files::pane_note(
                    &bear::empty_note(error.as_deref(), available),
                    p,
                ))
                .into_any_element();
        }
        let rows: Vec<AnyElement> = tabs
            .iter()
            .map(|tab| self.browser_row(tab, &thread.id, &browser_name, &model, cx))
            .collect();
        let pages = div()
            .id("browser-pages")
            .overflow_y_scroll()
            .child(v_flex().gap(px(8.)).p(px(10.)).children(rows));
        pane = pane.child(
            with_scroll_bar(pages, &self.browser.scroll, "browser-pages-bar")
                .flex_1()
                .min_h_0(),
        );
        if let Some(error) = error {
            pane = pane.child(
                div()
                    .flex_none()
                    .p(px(6.))
                    .text_size(px(style::size::LABEL))
                    .text_color(p.secondary)
                    .child(error),
            );
        }
        pane.into_any_element()
    }

    /// One page: its still, what it is, what the agent's doing in it, and the
    /// buttons under them, so a narrow pane never cuts a button's name short.
    fn browser_row(
        &self,
        tab: &AgentTab,
        thread: &str,
        browser_name: &str,
        model: &Entity<AgentBrowser>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = palette(cx);
        let b = model.read(cx);
        let yours = tab.yours();
        let amber = style::needs_you(cx);
        let dot = if yours {
            Some(amber)
        } else if tab.working() {
            Some(style::tick_green(cx))
        } else {
            None
        };
        let still = b.stills.get(&tab.id()).cloned();
        let (w, h) = (STILL_WIDTH, STILL_WIDTH * 0.62);
        let picture = div()
            .flex_none()
            .w(px(w))
            .h(px(h))
            .rounded(px(5.))
            .overflow_hidden()
            .bg(style::primary_opacity(cx, 0.06))
            .flex()
            .items_center()
            .justify_center();
        let picture = match still {
            Some(still) => tipped_as(
                picture
                    .id(SharedString::from(format!("browser-still-{}", tab.id())))
                    .child(
                        // Through the picture budget (fit_picture.rs): a new
                        // still every few seconds, and GPUI's own cache would
                        // keep every old one's pixels.
                        img(crate::fit_picture::bytes_source(still.image))
                            .size_full()
                            .object_fit(ObjectFit::Cover),
                    ),
                still_tip(still.at),
                gpui_kit::Role::Image,
            )
            .into_any_element(),
            None => picture
                .child(icon("globe", 16., p.faint))
                .into_any_element(),
        };
        let mut words = v_flex().flex_1().min_w_0().gap(px(3.)).child(
            h_flex()
                .items_center()
                .gap(px(6.))
                .when_some(dot, |s, c| {
                    s.child(div().flex_none().size(px(7.)).rounded_full().bg(c))
                })
                .child(
                    div()
                        .min_w_0()
                        .line_clamp(2)
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(px(style::size::LABEL))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.fg)
                        .child(tab.shown_title()),
                ),
        );
        words = words.child(
            div()
                .truncate()
                .text_size(px(style::size::LABEL))
                .text_color(p.secondary)
                .child(tab.host()),
        );
        if let (true, Some(action)) = (tab.working(), &tab.action) {
            words = words.child(
                div()
                    .truncate()
                    .text_size(px(style::size::LABEL))
                    .text_color(p.secondary)
                    .child(action.clone()),
            );
        }
        // Only the agents' own browser's windows are placed.
        let beside = tab.profile_id() == bear::AGENTS_BROWSER
            && self
                .app
                .upgrade()
                .is_some_and(|a| a.read(cx).settings.browser_beside());
        let (show_tab, show_thread, show_model) = (tab.clone(), thread.to_owned(), model.clone());
        let show = PushButton::new(
            SharedString::from(format!("browser-show-{}", tab.id())),
            "Show window",
            format!(
                "Bring this page's window to the front, in {}",
                tab.place(browser_name)
            ),
        )
        .small()
        .busy(b.pending.contains(&pending_key("panelShow", tab)))
        .render(cx, move |_, window, cx| {
            let place = beside.then(|| place_beside(window, cx)).flatten();
            show_model.update(cx, |b, cx| b.show(&show_tab, &show_thread, place, cx))
        });
        let picking_this = b
            .active_pick
            .as_ref()
            .is_some_and(|pick| pick.tab_key == tab.id() && pick.thread_id == thread);
        let (pick_tab, pick_thread, pick_model, pick_client) = (
            tab.clone(),
            thread.to_owned(),
            model.clone(),
            self.client.clone(),
        );
        let pick_button = PushButton::new(
            SharedString::from(format!("browser-pick-{}", tab.id())),
            if picking_this {
                "Cancel pick"
            } else {
                "Select on page"
            },
            if picking_this {
                CANCEL_PICK_TIP
            } else {
                PICK_BUTTON_TIP
            },
        )
        .small()
        .busy(b.pending.contains(&pending_key("panelPick", tab)))
        .render(cx, move |_, window, cx| {
            let place = beside.then(|| place_beside(window, cx)).flatten();
            pick_model.update(cx, |b, cx| {
                if picking_this {
                    b.cancel_pick(cx);
                } else {
                    b.start_pick(
                        &pick_tab,
                        &pick_thread,
                        "element",
                        (place, beside),
                        pick_client.clone(),
                        cx,
                    );
                }
            })
        });
        let mut buttons = h_flex()
            .flex_wrap()
            .gap(px(6.))
            .child(show)
            .child(pick_button);
        if yours {
            let (done_tab, done_model) = (tab.clone(), model.clone());
            buttons = buttons.child(
                PushButton::new(
                    SharedString::from(format!("browser-done-{}", tab.id())),
                    "Done",
                    "Tell the agent you've finished here",
                )
                .small()
                .busy(b.pending.contains(&pending_key("markDone", tab)))
                .render(cx, move |_, _, cx| {
                    done_model.update(cx, |b, cx| b.done(&done_tab, cx))
                }),
            );
        }
        let edge = if yours {
            amber.opacity(0.6)
        } else {
            style::primary_opacity(cx, 0.08)
        };
        v_flex()
            .gap(px(8.))
            .p(px(8.))
            .rounded(px(8.))
            .border_1()
            .border_color(edge)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(10.))
                    .child(picture)
                    .child(words),
            )
            .child(buttons)
            .into_any_element()
    }

    /// What the dump says of the pane: each page as a line.
    #[cfg(feature = "automation")]
    pub(crate) fn fill_browser_dump(
        &self,
        pane: &mut serde_json::Map<String, Value>,
        thread: &ThreadSummary,
        cx: &App,
    ) {
        let Some(model) = model(cx) else { return };
        let b = model.read(cx);
        let tabs = b.tabs_for(&thread.id);
        if tabs.is_empty() {
            pane.insert(
                "empty".into(),
                json!(bear::empty_note(b.error.as_deref(), b.available())),
            );
            return;
        }
        let rows: Vec<Value> = tabs
            .iter()
            .map(|t| {
                let picking_this = b
                    .active_pick
                    .as_ref()
                    .is_some_and(|pick| pick.tab_key == t.id() && pick.thread_id == thread.id);
                let mut buttons = vec![
                    "Show window",
                    if picking_this {
                        "Cancel pick"
                    } else {
                        "Select on page"
                    },
                ];
                if t.yours() {
                    buttons.push("Done");
                }
                json!({ "title": t.shown_title(), "host": t.host(), "state": t.state, "action": t.working().then(|| t.action.clone()).flatten(), "buttons": buttons, "still": b.stills.contains_key(&t.id()) })
            })
            .collect();
        pane.insert("pages".into(), json!(rows));
        if let Some(pick) = &b.active_pick {
            pane.insert(
                "picking".into(),
                json!({ "captureId": pick.capture_id, "thread": pick.thread_id, "tab": pick.tab_title, "mode": pick.mode }),
            );
        }
        if let Some(note) = &b.pick_note {
            pane.insert("pickNote".into(), json!(note));
        }
        if let Some(error) = &b.error {
            pane.insert("error".into(), json!(error));
        }
    }
}

/// "As the page looked 2 minutes ago".
fn still_tip(at: Option<f64>) -> String {
    let Some(at) = at else {
        return "As the page looked lately".into();
    };
    let ago = ((crate::now_ms() as f64 - at) / 1000.).max(0.);
    let when = match ago as u64 {
        0..=9 => "just now".to_owned(),
        s @ 10..=59 => format!("{s} seconds ago"),
        60..=119 => "a minute ago".to_owned(),
        s @ 120..=7199 => format!("{} minutes ago", s / 60),
        s => format!("{} hours ago", s / 3600),
    };
    format!("As the page looked {when}")
}

// ---------------------------------------------------------------- the rail

impl crate::app::Threadbear {
    /// The rail's Browser button: the count of the thread's pages, and the
    /// accent dot while one's handed to Mike. None without BrowserBear.
    pub(crate) fn browser_rail(&self, thread: &str, cx: &App) -> Option<(usize, bool)> {
        let model = model(cx)?;
        let b = model.read(cx);
        if !b.available() {
            return None;
        }
        let tabs = b.tabs_for(thread);
        Some((tabs.len(), tabs.iter().any(AgentTab::yours)))
    }

    /// The agent handing a page over is the one time the pane opens by itself,
    /// and coming back to a thread with a page still Mike's turn opens it
    /// again: the runner's one-shot show_panel can be cleared by a message,
    /// and the waiting tab is what counts (`PanelRail`'s onChange and onAppear).
    pub(crate) fn browser_changed(&mut self, cx: &mut Context<Self>) {
        // A project's page isn't a thread: coming back from it counts as
        // coming back to the thread, as Swift's rail appearing again does.
        if self.projects.open.is_some() {
            self.browser_handoff_seen = None;
            return;
        }
        let Some(thread) = self.pane_thread() else {
            return;
        };
        let Some(model) = model(cx) else { return };
        let key = bear::handoff_key(&model.read(cx).tabs, &thread.id);
        if key == self.browser_handoff_seen {
            return;
        }
        self.browser_handoff_seen = key.clone();
        if key.is_some() && !self.dock.read(cx).is_showing(BROWSER) {
            // Once this update is over: choosing a section can leave a file
            // or bring a preview back, and both read the app (18 Nov).
            let (side, id) = (self.side.clone(), thread.id);
            cx.defer(move |cx| side.update(cx, |side, cx| side.choose_section(&id, BROWSER, cx)));
            self.show_pane(BROWSER, cx);
        }
    }
}
