//! BrowserBear (github.com/mikecann/browserbear), the agents' browser, as the
//! app sees it: the Browser pane's pages and the sign-in handoff
//! (docs/protocol/panels.md, "The Browser panel" and "Showing panels"). A port of the
//! Swift app's `AgentBrowserPanel.swift`, `BrowserBear.swift` and
//! `BrowserBearPipe.swift`; the socket itself is the GPUI app's
//! (`agent_browser.rs`), this is what's said over it and what it means.
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The dock's id for the pane section, as the Swift app's `AgentBrowser.panelId`.
pub const PANEL_ID: &str = "threadbear.browser";

/// BrowserBear's id for the agents' own browser, as opposed to one of Mike's
/// Chrome profiles.
pub const AGENTS_BROWSER: &str = "agent-chrome";

/// BrowserBear's folder: `BROWSERBEAR_HOME`, else its usual place. A scripted
/// app (the specs, an agent's dev-app) gets none without `BROWSERBEAR_HOME`,
/// so it never reaches the real BrowserBear on the Mac running it: a click
/// there would bring Mike's real browser windows forward.
pub fn home(env_home: Option<&str>, user_home: &Path, scripted: bool) -> Option<PathBuf> {
    match env_home.filter(|h| !h.is_empty()) {
        Some(h) => Some(PathBuf::from(h)),
        None if scripted => None,
        None => Some(user_home.join("Library/Application Support/BrowserBear")),
    }
}

pub fn socket(home: &Path) -> PathBuf {
    home.join("browserbear.sock")
}

/// What BrowserBear's agent host writes about the agents' browser.
pub fn tabs_file(home: &Path) -> PathBuf {
    home.join("agent/tabs.json")
}

/// A page a thread's agent is using (`AgentTab`). Every field but the tab's
/// number and page is optional, as hosts add fields over time.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentTab {
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub tab_id: i64,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub target_id: Option<String>,
    /// In one of Mike's own Chrome profiles rather than the agents' browser.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub bridge: Option<bool>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub profile: Option<String>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub profile_name: Option<String>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub profile_email: Option<String>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub url: String,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub title: String,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub thread: Option<String>,
    /// working, yours or idle (BrowserBear's service/marks.mjs); none when unmarked.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub state: Option<String>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub action: Option<String>,
    /// The agent is waiting on `handoff --wait`, so Done means something.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub finish: Option<bool>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub status: Option<String>,
    /// Goes up each time the agent hands this tab to Mike.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub handoffs: Option<i64>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub handoff_at: Option<f64>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub popup: Option<bool>,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub opened_at: Option<f64>,
}

impl AgentTab {
    /// The profile it's in: the same tab number can be in two.
    pub fn profile_id(&self) -> &str {
        self.profile.as_deref().unwrap_or(AGENTS_BROWSER)
    }

    pub fn id(&self) -> String {
        format!("{}/{}", self.profile_id(), self.tab_id)
    }

    /// Handed to Mike: amber, first in the list, and Done.
    pub fn yours(&self) -> bool {
        self.state.as_deref() == Some("yours")
    }

    pub fn working(&self) -> bool {
        self.state.as_deref() == Some("working")
    }

    /// "Personal · me@example.com".
    pub fn profile_title(&self) -> String {
        [&self.profile_name, &self.profile_email]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// The page's host, or the whole address when it hasn't one.
    pub fn host(&self) -> String {
        host(&self.url).unwrap_or_else(|| self.url.clone())
    }

    /// Its title, or the host for a page without one yet.
    pub fn shown_title(&self) -> String {
        if self.title.is_empty() {
            self.host()
        } else {
            self.title.clone()
        }
    }

    /// Where its window is: a Chrome profile's name, else the agents' browser.
    pub fn place(&self, browser_name: &str) -> String {
        if self.bridge == Some(true) {
            let title = self.profile_title();
            if title.is_empty() {
                "Chrome".to_owned()
            } else {
                title
            }
        } else {
            browser_name.to_owned()
        }
    }
}

/// `scheme://host[:port]/...`'s host, lower-cased by nobody: as written.
fn host(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if authority.starts_with('[') {
        authority.split_inclusive(']').next()?
    } else {
        authority.split(':').next()?
    };
    (!host.is_empty()).then(|| host.to_owned())
}

/// The agent host's tabs.json.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct AgentState {
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub port: i64,
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub socket: String,
    /// The browser it runs: Helium, or Google Chrome without it.
    #[serde(default, deserialize_with = "crate::wire::field")]
    pub browser: Option<String>,
    #[serde(deserialize_with = "capped_tabs")]
    pub tabs: Vec<AgentTab>,
}

/// The first `MAX_TABS` pages, the rest passed over unbuilt: a few MB of
/// `{}`s would otherwise be a million pages made and thrown away.
fn capped_tabs<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<AgentTab>, D::Error> {
    struct Capped;
    impl<'de> serde::de::Visitor<'de> for Capped {
        type Value = Vec<AgentTab>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a list of pages")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut tabs = vec![];
            for _ in 0..MAX_TABS {
                match seq.next_element::<Value>()? {
                    Some(value) => {
                        if let Some(tab) = crate::wire::read(value) {
                            tabs.push(tab);
                        }
                    }
                    None => return Ok(tabs),
                }
            }
            while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
            Ok(tabs)
        }
        // A wrongly typed tabs field leaves the host's other fields usable.
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            while map
                .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                .is_some()
            {}
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            crate::wire::dropped::<AgentState>("tabs field");
            Ok(vec![])
        }
    }
    d.deserialize_any(Capped)
}

/// The biggest tabs.json read: a few hundred pages is a few hundred KB.
pub const MAX_TABS_FILE: u64 = 4 * 1024 * 1024;
/// The most pages kept from either place.
pub const MAX_TABS: usize = 2000;

pub fn agent_state(bytes: &[u8]) -> Option<AgentState> {
    serde_json::from_slice(bytes).ok()
}

/// `panelState`'s answer: the connected Chrome profiles and their pages.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct PanelState {
    #[serde(deserialize_with = "capped_tabs")]
    pub tabs: Vec<AgentTab>,
}

/// The agents' browser's pages and the Chrome profiles' (`mergeTabs`): one
/// each, a profile's winning a clash, in tab order.
/// A window's place on the screens, in points from the main screen's top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// The narrowest a page's window is worth placing beside the app's.
pub const BESIDE_MIN_WIDTH: f64 = 480.;
/// And the widest, as BrowserBear opens them.
pub const BESIDE_MAX_WIDTH: f64 = 1280.;

/// Where a page's window goes beside the app's `window` on its `screen`
/// (Settings > General > Browser): against whichever side has more room, as
/// tall as the app's window. None when neither side has room for a usable
/// page (the app's window is wide or full screen): then nothing's asked, and
/// the page's window stays where it is rather than the app's being moved.
pub fn beside(window: Place, screen: Place) -> Option<Place> {
    let right = screen.left + screen.width - (window.left + window.width);
    let left = window.left - screen.left;
    let room = right.max(left);
    if !room.is_finite() || room < BESIDE_MIN_WIDTH {
        return None;
    }
    let width = room.min(BESIDE_MAX_WIDTH).floor();
    let top = window.top.max(screen.top);
    let height = (window.top + window.height).min(screen.top + screen.height) - top;
    if height.is_nan() || height < 300. {
        return None;
    }
    Some(Place {
        left: if right >= left {
            window.left + window.width
        } else {
            window.left - width
        },
        top,
        width,
        height: height.floor(),
    })
}

/// The narrowest the app's window is made to fit a page's beside it (its own
/// least width).
pub const APP_MIN_WIDTH: f64 = 760.;

/// When `beside` has no room: how wide the app's `window` should become (its
/// left edge kept) and where the page's window then goes, to its right. The
/// page gets a little under half of what the two share. None when the screen
/// can't hold both at a usable width, or the numbers make no sense.
pub fn make_room(window: Place, screen: Place) -> Option<(f64, Place)> {
    let left = window.left.max(screen.left);
    let span = screen.left + screen.width - left;
    if !(window.left.is_finite() && window.width.is_finite() && span.is_finite())
        || span < APP_MIN_WIDTH + BESIDE_MIN_WIDTH
    {
        return None;
    }
    let page = (span * 0.45)
        .clamp(BESIDE_MIN_WIDTH, BESIDE_MAX_WIDTH)
        .min(span - APP_MIN_WIDTH)
        .floor();
    // The app's window fills what's left, but never grows.
    let app = (span - page).min(window.width);
    let place = beside(
        Place {
            left,
            width: app,
            ..window
        },
        screen,
    )?;
    Some((app, place))
}

impl Place {
    /// As BrowserBear takes it (`place` in panelShow and panelPick).
    pub fn json(&self) -> Value {
        json!({ "left": self.left.round(), "top": self.top.round(), "width": self.width, "height": self.height })
    }
}

#[cfg(test)]
mod beside_tests {
    use super::*;
    const SCREEN: Place = Place {
        left: 0.,
        top: 25.,
        width: 1728.,
        height: 1092.,
    };
    fn win(left: f64, width: f64) -> Place {
        Place {
            left,
            top: 60.,
            width,
            height: 900.,
        }
    }

    #[test]
    fn goes_against_the_side_with_more_room() {
        let got = beside(win(40., 1000.), SCREEN).unwrap();
        assert_eq!(
            got,
            Place {
                left: 1040.,
                top: 60.,
                width: 688.,
                height: 900.
            }
        );
        let got = beside(win(700., 1000.), SCREEN).unwrap();
        assert_eq!(
            got,
            Place {
                left: 0.,
                top: 60.,
                width: 700.,
                height: 900.
            }
        );
    }

    #[test]
    fn is_no_wider_than_browserbear_opens_them() {
        let wide = Place {
            left: 0.,
            top: 0.,
            width: 3440.,
            height: 1440.,
        };
        let got = beside(
            Place {
                left: 0.,
                top: 0.,
                width: 1000.,
                height: 1440.,
            },
            wide,
        )
        .unwrap();
        assert_eq!((got.left, got.width), (1000., BESIDE_MAX_WIDTH));
    }

    #[test]
    fn makes_room_when_the_app_fills_the_screen() {
        let full = Place {
            left: 0.,
            top: 33.,
            width: 1512.,
            height: 949.,
        };
        let screen = Place {
            left: 0.,
            top: 0.,
            width: 1512.,
            height: 982.,
        };
        assert_eq!(beside(full, screen), None);
        let (app, page) = make_room(full, screen).unwrap();
        assert_eq!(app, 832.);
        assert_eq!(
            page,
            Place {
                left: 832.,
                top: 33.,
                width: 680.,
                height: 949.
            }
        );
        // Both still fit on the smallest screen that can hold them.
        let small = Place {
            width: 1240.,
            ..screen
        };
        let (app, page) = make_room(
            Place {
                width: 1240.,
                ..full
            },
            small,
        )
        .unwrap();
        assert_eq!((app, page.left, page.width), (760., 760., 480.));
        // And not on one that can't.
        let tiny = Place {
            width: 1200.,
            ..screen
        };
        assert_eq!(
            make_room(
                Place {
                    width: 1200.,
                    ..full
                },
                tiny
            ),
            None
        );
        assert_eq!(
            make_room(
                Place {
                    left: f64::NAN,
                    ..full
                },
                screen
            ),
            None
        );
    }

    #[test]
    fn makes_room_from_where_the_window_starts() {
        // 300 free on each side: too little for a page, so the app narrows
        // and keeps its left edge.
        let (app, page) = make_room(win(300., 1128.), SCREEN).unwrap();
        assert_eq!(page.width, 642.);
        assert_eq!(app, 1428. - 642.);
        assert_eq!(page.left, 300. + app);
        assert_eq!(page.left + page.width, 1728.);
    }

    #[test]
    fn asks_nothing_without_room_or_sense() {
        assert_eq!(beside(win(100., 1500.), SCREEN), None);
        assert_eq!(beside(SCREEN, SCREEN), None);
        assert_eq!(beside(win(f64::NAN, 800.), SCREEN), None);
        let short = Place {
            left: 0.,
            top: 900.,
            width: 800.,
            height: 2000.,
        };
        assert_eq!(beside(short, SCREEN), None);
    }

    #[test]
    fn works_on_a_second_screen_to_the_left() {
        let second = Place {
            left: -1920.,
            top: 0.,
            width: 1920.,
            height: 1080.,
        };
        let got = beside(
            Place {
                left: -1900.,
                top: 40.,
                width: 900.,
                height: 2000.,
            },
            second,
        )
        .unwrap();
        assert_eq!(
            got,
            Place {
                left: -1000.,
                top: 40.,
                width: 1000.,
                height: 1040.
            }
        );
    }
}

pub fn merge(agent: &[AgentTab], profiles: &[AgentTab]) -> Vec<AgentTab> {
    let mut all: std::collections::HashMap<String, AgentTab> = std::collections::HashMap::new();
    for tab in agent.iter().take(MAX_TABS).chain(
        profiles
            .iter()
            .filter(|t| t.bridge == Some(true))
            .take(MAX_TABS),
    ) {
        all.insert(tab.id(), tab.clone());
    }
    let mut all: Vec<AgentTab> = all.into_values().collect();
    all.sort_by(|a, b| {
        a.tab_id
            .cmp(&b.tab_id)
            .then_with(|| a.profile_id().cmp(b.profile_id()))
    });
    all
}

/// A thread's pages as the pane lists them: waiting for Mike first, then the newest.
pub fn for_thread(tabs: &[AgentTab], thread: &str) -> Vec<AgentTab> {
    let mut mine: Vec<AgentTab> = tabs
        .iter()
        .filter(|t| t.thread.as_deref() == Some(thread))
        .cloned()
        .collect();
    let newest = |t: &AgentTab| t.opened_at.unwrap_or(t.tab_id as f64);
    mine.sort_by(|a, b| {
        b.yours()
            .cmp(&a.yours())
            .then(newest(b).total_cmp(&newest(a)))
    });
    mine
}

/// What changes when an agent hands one of the thread's pages to Mike (again):
/// the pane opens by itself when this changes and has a page that's his.
/// None while nothing is his.
pub fn handoff_key(tabs: &[AgentTab], thread: &str) -> Option<String> {
    let yours: Vec<String> = tabs
        .iter()
        .filter(|t| t.thread.as_deref() == Some(thread) && t.yours())
        .map(|t| format!("{}:{}", t.id(), t.handoffs.unwrap_or(0)))
        .collect();
    (!yours.is_empty()).then(|| format!("{thread}|{}", yours.join(",")))
}

/// What's typed in the pane's address bar as a page to open: an address as
/// it is, a bare name with a dot as https, anything else a search.
pub fn address(typed: &str) -> Option<String> {
    let text = typed.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains("://") {
        return Some(text.to_owned());
    }
    if text.contains('.') && !text.contains(' ') {
        return Some(format!("https://{text}"));
    }
    let query: String = text
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_owned(),
            other => format!("%{other:02X}"),
        })
        .collect();
    Some(format!("https://www.google.com/search?q={query}"))
}

/// Whether a handoff's reply says the tab came to Mike: its own id and an
/// explicit `ok: true`. Anything else opens the link in the browser as usual.
pub fn handed_off(reply: &[u8], id: u64) -> bool {
    let Ok(message) = serde_json::from_slice::<Value>(reply) else {
        return false;
    };
    message["id"].as_u64() == Some(id) && message["ok"].as_bool() == Some(true)
}

/// Whether a link could be one an agent has open: only web pages are asked about.
pub fn handoff_candidate(url: &str) -> bool {
    let scheme = url.split(':').next().unwrap_or("").to_ascii_lowercase();
    scheme == "http" || scheme == "https"
}

/// One request line: `{id, cmd, args[, thread]}` and a newline.
pub fn request_line(id: u64, cmd: &str, args: Value, thread: Option<&str>) -> Vec<u8> {
    let mut message = json!({ "id": id, "cmd": cmd, "args": args });
    if let Some(thread) = thread {
        message["thread"] = json!(thread);
    }
    let mut line = serde_json::to_vec(&message).unwrap_or_default();
    line.push(b'\n');
    line
}

/// One line back.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// `{id, ok, result|error}`: the result, or what went wrong.
    Reply {
        id: u64,
        answer: Result<Value, String>,
    },
    /// Anything without an id.
    Event(Value),
}

/// None for a line that isn't JSON, which ends the connection, as Swift's does.
pub fn read_line(line: &[u8]) -> Option<Line> {
    let message: Value = serde_json::from_slice(line).ok()?;
    if !message.is_object() {
        return None;
    }
    let Some(id) = message["id"].as_u64() else {
        return Some(Line::Event(message));
    };
    let answer = if message["ok"].as_bool() == Some(false) {
        Err(message["error"]
            .as_str()
            .unwrap_or("BrowserBear couldn't open this page.")
            .to_owned())
    } else {
        Ok(match &message["result"] {
            Value::Object(_) => message["result"].clone(),
            _ => json!({}),
        })
    };
    Some(Line::Reply { id, answer })
}

/// The biggest line kept waiting for its end: past it the other side isn't
/// BrowserBear, or is broken.
pub const MAX_LINE: usize = 16 * 1024 * 1024;

/// Whole lines out of reads that split them anywhere (a still is hundreds of
/// KB and comes in many reads; several short lines can share one).
#[derive(Debug, Default)]
pub struct Lines {
    buffer: Vec<u8>,
}

impl Lines {
    /// The lines this read finished, oldest first; Err when one is too long.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        let from = self.buffer.len();
        self.buffer.extend_from_slice(bytes);
        let mut lines = vec![];
        let mut start = 0;
        // Only what's new can hold a line's end.
        let mut search = from;
        while let Some(at) = self.buffer[search..].iter().position(|b| *b == b'\n') {
            let end = search + at;
            lines.push(self.buffer[start..end].to_vec());
            start = end + 1;
            search = start;
        }
        self.buffer.drain(..start);
        if self.buffer.len() > MAX_LINE {
            return Err("BrowserBear sent an oversized frame.".to_owned());
        }
        Ok(lines)
    }
}

/// `panelStill`'s answer: a JPEG and when BrowserBear took it (ms since 1970).
pub fn still(result: &Value) -> Option<(Vec<u8>, Option<f64>)> {
    use base64::Engine as _;
    let data = result["data"].as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    Some((bytes, result["at"].as_f64()))
}

/// What the pane says with no pages to list.
pub fn empty_note(error: Option<&str>, available: bool) -> String {
    match error {
        Some(e) => e.to_owned(),
        None if available => "No pages open in this thread.".to_owned(),
        None => "BrowserBear isn't running.".to_owned(),
    }
}

// ---------------------------------------------------------------- Add from page

/// Conservative cap for a decoded pick screenshot when no tighter runner limit
/// rejects it first (`media.import` still enforces the runner's own cap).
pub const MAX_PICK_SCREENSHOT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PICK_QUOTE_CHARS: usize = 4000;
const MAX_URL_LEN: usize = 2000;

const SECRET_KEYS: &[&str] = &[
    "token",
    "auth",
    "authorization",
    "code",
    "session",
    "sid",
    "key",
    "apikey",
    "password",
    "pwd",
    "pass",
    "passwd",
    "secret",
    "signature",
    "sig",
    "jwt",
    "otp",
    "state",
    "nonce",
    "sessionid",
    "authtoken",
    "bearer",
    "credential",
    "credentials",
    "api_key",
    "access_key",
    "private_key",
    "secret_key",
    "auth_key",
];

const SECRET_WORDS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "pwd",
    "signature",
    "credential",
    "credentials",
    "session",
    "sessionid",
    "phpsessid",
    "jsessionid",
    "apikey",
    "auth",
    "authorization",
    "jwt",
    "otp",
    "bearer",
    "csrf",
    "xsrf",
];

const SECRET_KEY_PREFIXES: &[&str] = &[
    "api", "access", "secret", "private", "auth", "session", "client", "signing",
];

fn normalize_param_key(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_alnum_lower = false;
    let mut prev_sep = false;
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            if prev_alnum_lower && !prev_sep {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_alnum_lower = false;
            prev_sep = false;
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_alnum_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            prev_sep = false;
        } else if !prev_sep && !out.is_empty() {
            out.push('_');
            prev_alnum_lower = false;
            prev_sep = true;
        }
    }
    out.trim_matches('_').to_owned()
}

fn is_secret_key(name: &str) -> bool {
    let key = normalize_param_key(name);
    if SECRET_KEYS.contains(&key.as_str()) {
        return true;
    }
    let words: Vec<&str> = key.split('_').filter(|w| !w.is_empty()).collect();
    if words.iter().any(|w| SECRET_WORDS.contains(w)) {
        return true;
    }
    for pair in words.windows(2) {
        if SECRET_KEY_PREFIXES.contains(&pair[0]) && pair[1] == "key" {
            return true;
        }
    }
    words
        .iter()
        .any(|w| w.ends_with("key") && SECRET_KEY_PREFIXES.contains(&&w[..w.len() - 3]))
}

fn percent_decode_key(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Strips credentials, token-bearing query parameters, and query-like fragments
/// from a captured page URL. Non-http(s) addresses come back empty.
/// Returns `(sanitized_url, removed_something)`.
pub fn sanitize_url(raw: &str) -> (String, bool) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (String::new(), false);
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return (String::new(), true);
    };
    let scheme_lower = scheme.to_ascii_lowercase();
    if scheme_lower != "http" && scheme_lower != "https" {
        return (String::new(), true);
    }
    let (without_hash, hash) = match rest.split_once('#') {
        Some((before, h)) => (before, Some(h)),
        None => (rest, None),
    };
    let (before_query, query) = match without_hash.split_once('?') {
        Some((before, q)) => (before, Some(q)),
        None => (without_hash, None),
    };
    let (authority, path) = match before_query.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (before_query, String::new()),
    };
    if authority.is_empty() {
        return (String::new(), true);
    }
    let (clean_authority, mut removed) = match authority.rsplit_once('@') {
        Some((_, host_port)) => (host_port, true),
        None => (authority, false),
    };
    if clean_authority.is_empty() {
        return (String::new(), true);
    }
    let origin_path = format!("{scheme_lower}://{clean_authority}{path}");
    let mut kept_query = Vec::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let key_raw = pair.split_once('=').map_or(pair, |(k, _)| k);
            let key_decoded = percent_decode_key(key_raw);
            if is_secret_key(&key_decoded) {
                removed = true;
            } else {
                kept_query.push(pair);
            }
        }
    }
    let mut out = origin_path.clone();
    if !kept_query.is_empty() {
        out.push('?');
        out.push_str(&kept_query.join("&"));
    }
    if let Some(h) = hash {
        if h.contains('=') || h.contains('&') {
            removed = true;
        } else if !h.is_empty() {
            out.push('#');
            out.push_str(h);
        }
    }
    if out.len() > MAX_URL_LEN {
        out = origin_path.chars().take(MAX_URL_LEN).collect();
        removed = true;
    }
    (out, removed)
}

fn cap_text(value: Option<&str>, max_chars: usize) -> Option<String> {
    let s = value?.trim();
    if s.is_empty() {
        return None;
    }
    Some(s.chars().take(max_chars).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickImageFormat {
    Png,
    Jpeg,
}

impl PickImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            PickImageFormat::Png => "png",
            PickImageFormat::Jpeg => "jpg",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickScreenshot {
    pub bytes: Vec<u8>,
    pub format: PickImageFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PickStyles {
    pub font: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub radius: Option<String>,
    pub padding: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PickTarget {
    pub tag: Option<String>,
    pub role: Option<String>,
    pub name: Option<String>,
    pub selector: Option<String>,
    pub styles: Option<PickStyles>,
}

/// One captured element, region, or quote from BrowserBear, already sanitized
/// and validated against its pinned thread and `capture_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickCapture {
    pub capture_id: String,
    pub thread: String,
    pub created_at: Option<i64>,
    pub kind: String,
    pub source_title: String,
    pub source_url: String,
    pub screenshot: Option<PickScreenshot>,
    pub quote: Option<String>,
    pub target: Option<PickTarget>,
    pub warnings: Vec<String>,
}

impl PickCapture {
    pub fn source_host(&self) -> String {
        host(&self.source_url).unwrap_or_else(|| self.source_url.clone())
    }

    pub fn display_title(&self) -> String {
        if !self.source_title.is_empty() {
            self.source_title.clone()
        } else if !self.source_url.is_empty() {
            self.source_host()
        } else {
            "Page capture".to_owned()
        }
    }

    /// One compact line describing the picked target for collapsed metadata.
    pub fn metadata_summary(&self) -> Option<String> {
        let t = self.target.as_ref()?;
        let what = t.role.as_deref().or(t.tag.as_deref()).unwrap_or("element");
        let mut parts = vec![match &t.name {
            Some(name) => format!("{what} \"{name}\""),
            None => what.to_owned(),
        }];
        if let Some(sel) = &t.selector {
            parts.push(sel.clone());
        }
        Some(parts.join(" · "))
    }

    /// Plain-text key/value rows for the expanded metadata section.
    pub fn metadata_details(&self) -> Vec<(String, String)> {
        let Some(t) = self.target.as_ref() else {
            return vec![];
        };
        let mut rows = vec![];
        if let Some(tag) = &t.tag {
            rows.push(("Tag".into(), tag.clone()));
        }
        if let Some(role) = &t.role {
            rows.push(("Role".into(), role.clone()));
        }
        if let Some(name) = &t.name {
            rows.push(("Name".into(), name.clone()));
        }
        if let Some(sel) = &t.selector {
            rows.push(("Selector".into(), sel.clone()));
        }
        if let Some(styles) = &t.styles {
            let mut style_parts = vec![];
            if let Some(f) = &styles.font {
                style_parts.push(format!("font {f}"));
            }
            if let Some(c) = &styles.color {
                style_parts.push(format!("color {c}"));
            }
            if let Some(bg) = &styles.background {
                style_parts.push(format!("bg {bg}"));
            }
            if let Some(r) = &styles.radius {
                style_parts.push(format!("radius {r}"));
            }
            if let Some(p) = &styles.padding {
                style_parts.push(format!("padding {p}"));
            }
            if !style_parts.is_empty() {
                rows.push(("Styles".into(), style_parts.join(", ")));
            }
        }
        rows
    }

    /// Plain-word notices explaining target limitations (not including the
    /// always-shown screenshot secret warning).
    pub fn target_notes(&self) -> Vec<&'static str> {
        let mut out = vec![];
        for w in &self.warnings {
            let note = match w.as_str() {
                "iframe_unreadable" => {
                    "Can't select inside this embedded frame. Showing what is visible without its page details."
                }
                "closed_shadow_root" => {
                    "Can't target the inside of this control. Showing the host element, or pick again with Region."
                }
                "url_query_removed" => {
                    "Sensitive parameters were removed from the source address."
                }
                "cropped_to_view" => "Cropped to what was visible on screen.",
                "quote_truncated" => "Selected text was shortened to fit.",
                _ => continue,
            };
            if !out.contains(&note) {
                out.push(note);
            }
        }
        out
    }

    pub fn has_secret_warning(&self) -> bool {
        self.screenshot.is_some()
            || self
                .warnings
                .iter()
                .any(|w| w == "may_contain_visible_secrets")
    }

    /// Formats the capture as plain Markdown context attached when the user
    /// presses Send. Never executes or emits raw captured HTML.
    pub fn message_block(&self) -> String {
        let mut lines = vec![];
        let header = match (self.source_title.is_empty(), self.source_url.is_empty()) {
            (false, false) => format!("From page: [{}]({})", self.source_title, self.source_url),
            (true, false) => format!("From page: {}", self.source_url),
            (false, true) => format!("From page: {}", self.source_title),
            (true, true) => "From page:".to_owned(),
        };
        lines.push(header);
        if let Some(summary) = self.metadata_summary() {
            lines.push(format!("Element: {summary}"));
        }
        if let Some(quote) = &self.quote {
            let quoted = quote
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            lines.push(quoted);
        }
        lines.join("\n")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickState {
    Picking,
    Done,
    Cancelled,
    Gone,
    Replaced,
    Abandoned,
    Timeout,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickReply {
    pub capture_id: String,
    pub state: PickState,
    pub mode: Option<String>,
    pub note: Option<String>,
    pub capture: Option<PickCapture>,
}

fn parse_target(v: &Value) -> Option<PickTarget> {
    let obj = v.as_object()?;
    let styles = obj.get("styles").and_then(Value::as_object).and_then(|s| {
        let styles = PickStyles {
            font: cap_text(s.get("font").and_then(Value::as_str), 80),
            color: cap_text(s.get("color").and_then(Value::as_str), 80),
            background: cap_text(s.get("background").and_then(Value::as_str), 80),
            radius: cap_text(s.get("radius").and_then(Value::as_str), 40),
            padding: cap_text(s.get("padding").and_then(Value::as_str), 40),
        };
        (styles.font.is_some()
            || styles.color.is_some()
            || styles.background.is_some()
            || styles.radius.is_some()
            || styles.padding.is_some())
        .then_some(styles)
    });
    let target = PickTarget {
        tag: cap_text(obj.get("tag").and_then(Value::as_str), 40),
        role: cap_text(obj.get("role").and_then(Value::as_str), 40),
        name: cap_text(obj.get("name").and_then(Value::as_str), 80),
        selector: cap_text(obj.get("selector").and_then(Value::as_str), 300),
        styles,
    };
    (target.tag.is_some()
        || target.role.is_some()
        || target.name.is_some()
        || target.selector.is_some()
        || target.styles.is_some())
    .then_some(target)
}

fn parse_screenshot(v: &Value) -> Option<PickScreenshot> {
    use base64::Engine as _;
    let obj = v.as_object()?;
    let data = obj.get("data").and_then(Value::as_str)?;
    // Reject absurd base64 payloads before decoding.
    if data.len() > MAX_PICK_SCREENSHOT_BYTES * 2 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    if bytes.is_empty() || bytes.len() > MAX_PICK_SCREENSHOT_BYTES {
        return None;
    }
    let format = match obj.get("format").and_then(Value::as_str) {
        Some("jpeg") | Some("jpg") => PickImageFormat::Jpeg,
        _ => PickImageFormat::Png,
    };
    Some(PickScreenshot {
        bytes,
        format,
        width: obj.get("width").and_then(Value::as_u64).map(|n| n as u32),
        height: obj.get("height").and_then(Value::as_u64).map(|n| n as u32),
    })
}

/// Decodes a `panelPick` / `panelPickLook` reply and enforces `captureId` and
/// pinned `thread` matching. Returns `None` if the reply doesn't match the
/// in-flight session.
pub fn parse_pick_reply(
    result: &Value,
    expected_capture_id: &str,
    expected_thread: &str,
) -> Option<PickReply> {
    let capture_id = result["captureId"].as_str()?;
    if capture_id != expected_capture_id {
        return None;
    }
    let state = match result["state"].as_str()? {
        "picking" => PickState::Picking,
        "done" => PickState::Done,
        "cancelled" => PickState::Cancelled,
        "gone" => PickState::Gone,
        "replaced" => PickState::Replaced,
        "abandoned" => PickState::Abandoned,
        "timeout" => PickState::Timeout,
        "failed" => PickState::Failed,
        _ => PickState::Unknown,
    };
    let mode = cap_text(result["mode"].as_str(), 24);
    let note = cap_text(result["note"].as_str(), 400);
    let capture = if state == PickState::Done {
        let raw = result.get("capture")?.as_object()?;
        let cap_id = raw.get("captureId").and_then(Value::as_str)?;
        let cap_thread = raw.get("thread").and_then(Value::as_str)?;
        // Never accept a capture meant for another session or another thread.
        if cap_id != expected_capture_id || cap_thread != expected_thread {
            return None;
        }
        let raw_url = raw
            .get("source")
            .and_then(|s| s.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let (clean_url, removed) = sanitize_url(raw_url);
        let source_title = cap_text(
            raw.get("source")
                .and_then(|s| s.get("title"))
                .and_then(Value::as_str),
            200,
        )
        .unwrap_or_default();
        let screenshot = raw.get("screenshot").and_then(parse_screenshot);
        let quote = cap_text(
            raw.get("quote").and_then(Value::as_str),
            MAX_PICK_QUOTE_CHARS,
        );
        let target = raw.get("target").and_then(parse_target);
        let mut warnings: Vec<String> = raw
            .get("warnings")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .take(12)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if removed && !warnings.iter().any(|w| w == "url_query_removed") {
            warnings.push("url_query_removed".to_owned());
        }
        if screenshot.is_some() && !warnings.iter().any(|w| w == "may_contain_visible_secrets") {
            warnings.push("may_contain_visible_secrets".to_owned());
        }
        if screenshot.is_none() && quote.is_none() {
            return None;
        }
        Some(PickCapture {
            capture_id: cap_id.to_owned(),
            thread: cap_thread.to_owned(),
            created_at: raw.get("createdAt").and_then(Value::as_i64),
            kind: cap_text(raw.get("kind").and_then(Value::as_str), 24)
                .unwrap_or_else(|| "element".to_owned()),
            source_title,
            source_url: clean_url,
            screenshot,
            quote,
            target,
            warnings,
        })
    } else {
        None
    };
    Some(PickReply {
        capture_id: capture_id.to_owned(),
        state,
        mode,
        note,
        capture,
    })
}

/// Pure per-thread pending capture store: pins every capture to its own
/// `thread_id`, deduplicates by `capture_id`, and supports discarding or
/// stripping just the quote or screenshot before Send.
#[derive(Debug)]
pub struct CaptureStore<T> {
    by_thread: std::collections::HashMap<String, Vec<(PickCapture, T)>>,
    seen_ids: std::collections::HashSet<String>,
}

impl<T> Default for CaptureStore<T> {
    fn default() -> Self {
        Self {
            by_thread: std::collections::HashMap::new(),
            seen_ids: std::collections::HashSet::new(),
        }
    }
}

impl<T> CaptureStore<T> {
    /// Inserts a capture into its pinned thread if `capture_id` has not been
    /// accepted before. Returns `false` for a duplicate `capture_id`.
    pub fn insert(&mut self, capture: PickCapture, extra: T) -> bool {
        if !self.seen_ids.insert(capture.capture_id.clone()) {
            return false;
        }
        let thread = capture.thread.clone();
        self.by_thread
            .entry(thread)
            .or_default()
            .push((capture, extra));
        true
    }

    pub fn for_thread(&self, thread: &str) -> &[(PickCapture, T)] {
        self.by_thread.get(thread).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn for_thread_mut(&mut self, thread: &str) -> Option<&mut Vec<(PickCapture, T)>> {
        self.by_thread.get_mut(thread)
    }

    pub fn remove(&mut self, thread: &str, capture_id: &str) -> Option<(PickCapture, T)> {
        let list = self.by_thread.get_mut(thread)?;
        let pos = list.iter().position(|(c, _)| c.capture_id == capture_id)?;
        let item = list.remove(pos);
        if list.is_empty() {
            self.by_thread.remove(thread);
        }
        Some(item)
    }

    pub fn take_thread(&mut self, thread: &str) -> Vec<(PickCapture, T)> {
        self.by_thread.remove(thread).unwrap_or_default()
    }

    /// Puts back captures from a failed send without letting `seen_ids` block
    /// them.
    pub fn restore_thread(&mut self, thread: String, mut items: Vec<(PickCapture, T)>) {
        if items.is_empty() {
            return;
        }
        let entry = self.by_thread.entry(thread).or_default();
        items.append(entry);
        *entry = items;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(json: &str) -> AgentTab {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_threads_tabs_decode_from_the_hosts_file() {
        let t = tab(
            r#"{"tabId":1004,"targetId":"AB","url":"https://example.com/","title":"Example","thread":"t1","state":"yours","action":"over to you","finish":true,"status":"complete","handoffs":2,"openedAt":1,"popup":false}"#,
        );
        assert!(
            t.tab_id == 1004
                && t.thread.as_deref() == Some("t1")
                && t.yours()
                && t.handoffs == Some(2)
        );
    }

    #[test]
    fn missing_and_unknown_fields_are_fine() {
        let t =
            tab(r#"{"tabId":1,"targetId":"A","url":"about:blank","title":"","somethingNew":3}"#);
        assert!(t.thread.is_none() && t.handoffs.is_none());
        let state = agent_state(br#"{"port":9222,"socket":"/x","tabs":[{"tabId":1,"url":"https://a.com/","title":"A"}],"later":true}"#).unwrap();
        assert_eq!(
            (state.port, state.tabs.len(), state.browser),
            (9222, 1, None)
        );
        assert!(agent_state(b"not json").is_none());
    }

    #[test]
    fn existing_chrome_tabs_keep_their_profile_and_thread() {
        let t = tab(
            r#"{"tabId":24,"url":"https://github.com/","title":"GitHub","thread":"t1","bridge":true,"profile":"personal-id","profileName":"Personal","profileEmail":"me@example.com"}"#,
        );
        assert_eq!(t.profile_title(), "Personal · me@example.com");
        assert_eq!(t.id(), "personal-id/24");
        assert_eq!(t.place("Helium"), "Personal · me@example.com");
    }

    #[test]
    fn a_tab_in_the_agents_browser_is_kept_apart_from_one_in_chrome() {
        let agent = tab(r#"{"tabId":24,"url":"https://github.com/","title":"GitHub"}"#);
        assert_eq!(
            (agent.profile_id(), agent.id()),
            (AGENTS_BROWSER, "agent-chrome/24".to_owned())
        );
        let chrome = tab(
            r#"{"tabId":24,"url":"https://github.com/","title":"GitHub","bridge":true,"profile":"p"}"#,
        );
        // A profile tab not marked as bridged isn't one of the profiles' pages.
        let stray = tab(r#"{"tabId":25,"url":"https://x.com/","title":"X","profile":"p"}"#);
        let merged = merge(std::slice::from_ref(&agent), &[chrome, stray]);
        assert_eq!(
            merged.iter().map(AgentTab::id).collect::<Vec<_>>(),
            vec!["agent-chrome/24", "p/24"]
        );
    }

    #[test]
    fn a_threads_pages_put_its_turn_first_then_the_newest() {
        let tabs = vec![
            tab(
                r#"{"tabId":1,"url":"https://old.com/","title":"Old","thread":"t1","openedAt":10}"#,
            ),
            tab(
                r#"{"tabId":2,"url":"https://new.com/","title":"New","thread":"t1","openedAt":30}"#,
            ),
            tab(
                r#"{"tabId":3,"url":"https://login.com/","title":"Sign in","thread":"t1","state":"yours","openedAt":5}"#,
            ),
            tab(r#"{"tabId":4,"url":"https://other.com/","title":"Other","thread":"t2"}"#),
        ];
        let titles: Vec<String> = for_thread(&tabs, "t1")
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles, vec!["Sign in", "New", "Old"]);
    }

    #[test]
    fn the_handoff_key_moves_with_each_handoff_and_only_for_mikes_turn() {
        let mut tabs = vec![tab(
            r#"{"tabId":1,"url":"https://a.com/","title":"A","thread":"t1","state":"working"}"#,
        )];
        assert_eq!(handoff_key(&tabs, "t1"), None);
        tabs[0].state = Some("yours".into());
        tabs[0].handoffs = Some(1);
        let first = handoff_key(&tabs, "t1").unwrap();
        tabs[0].handoffs = Some(2);
        assert_ne!(
            handoff_key(&tabs, "t1").unwrap(),
            first,
            "handed over again"
        );
        assert_eq!(handoff_key(&tabs, "t2"), None, "another thread's");
    }

    #[test]
    fn the_address_bar_takes_addresses_names_and_searches() {
        assert_eq!(address("  "), None);
        assert_eq!(
            address("https://a.com/x").as_deref(),
            Some("https://a.com/x")
        );
        assert_eq!(address("qantas.com").as_deref(), Some("https://qantas.com"));
        assert_eq!(
            address("cheap flights & hotels").as_deref(),
            Some("https://www.google.com/search?q=cheap+flights+%26+hotels")
        );
    }

    #[test]
    fn hosts() {
        assert_eq!(
            tab(r#"{"tabId":1,"url":"https://user@www.qantas.com:443/login?x=1","title":""}"#)
                .shown_title(),
            "www.qantas.com"
        );
        assert_eq!(
            tab(r#"{"tabId":1,"url":"about:blank","title":""}"#).host(),
            "about:blank"
        );
    }

    #[test]
    fn only_a_plain_yes_to_this_request_counts_as_handed_off() {
        assert!(handed_off(br#"{"id":1,"ok":true,"result":{}}"#, 1));
        assert!(!handed_off(br#"{"id":1}"#, 1), "no ok");
        assert!(!handed_off(br#"{"id":1,"ok":"yes"}"#, 1));
        assert!(
            !handed_off(br#"{"id":2,"ok":true}"#, 1),
            "another request's"
        );
        assert!(!handed_off(br#"{"id":1,"ok":false,"error":"no tab"}"#, 1));
        assert!(!handed_off(b"nonsense", 1));
    }

    #[test]
    fn a_huge_list_merges_quickly_and_is_capped() {
        let many: Vec<AgentTab> = (0..100_000)
            .map(|i| AgentTab {
                tab_id: i,
                url: format!("https://a{i}.com/"),
                ..Default::default()
            })
            .collect();
        let started = std::time::Instant::now();
        assert_eq!(merge(&many, &many).len(), MAX_TABS);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn a_file_of_a_million_empty_pages_keeps_only_the_cap() {
        let mut text = String::from(r#"{"port":1,"tabs":["#);
        text.push_str(&vec!["{}"; 1_000_000].join(","));
        text.push_str("]}");
        let started = std::time::Instant::now();
        assert_eq!(agent_state(text.as_bytes()).unwrap().tabs.len(), MAX_TABS);
        assert!(started.elapsed() < std::time::Duration::from_secs(3));
    }

    #[test]
    fn only_web_pages_are_asked_about() {
        assert!(handoff_candidate("https://qantas.com/") && handoff_candidate("HTTP://a.com"));
        assert!(!handoff_candidate("file:///etc/passwd") && !handoff_candidate("mailto:a@b.com"));
    }

    #[test]
    fn requests_and_replies() {
        let line = request_line(
            7,
            "panelShow",
            json!({ "tab": 3, "profile": "agent-chrome" }),
            Some("t1"),
        );
        assert_eq!(*line.last().unwrap(), b'\n');
        let sent: Value = serde_json::from_slice(&line[..line.len() - 1]).unwrap();
        assert_eq!(
            sent,
            json!({ "id": 7, "cmd": "panelShow", "args": { "tab": 3, "profile": "agent-chrome" }, "thread": "t1" })
        );
        assert_eq!(
            read_line(br#"{"id":7,"ok":true,"result":{"tabs":[]}}"#),
            Some(Line::Reply {
                id: 7,
                answer: Ok(json!({ "tabs": [] }))
            })
        );
        assert_eq!(
            read_line(br#"{"id":8,"ok":false,"error":"no such tab"}"#),
            Some(Line::Reply {
                id: 8,
                answer: Err("no such tab".into())
            })
        );
        assert_eq!(
            read_line(br#"{"id":9,"ok":true}"#),
            Some(Line::Reply {
                id: 9,
                answer: Ok(json!({}))
            })
        );
        assert!(matches!(
            read_line(br#"{"method":"frame"}"#),
            Some(Line::Event(_))
        ));
        assert_eq!(read_line(b"garbage"), None);
    }

    /// Lines straddling reads, several in one read, and a reply after a big
    /// frame in the same read all come out whole and in order.
    #[test]
    fn lines_split_across_and_within_reads() {
        let big = "x".repeat(700_000);
        let text = format!("{{\"a\":\"{big}\"}}\n{{\"b\":1}}\n{{\"c\":\"{big}\"}}\n");
        let bytes = text.as_bytes();
        let mut lines = Lines::default();
        let mut got = vec![];
        let mut i = 0;
        for size in [5, 300_000, 400_036, 7, 650_000]
            .into_iter()
            .chain(std::iter::repeat(100_000))
        {
            if i >= bytes.len() {
                break;
            }
            let end = (i + size).min(bytes.len());
            got.extend(lines.push(&bytes[i..end]).unwrap());
            i = end;
        }
        let lens: Vec<usize> = got.iter().map(Vec::len).collect();
        assert_eq!(lens, vec![big.len() + 8, 7, big.len() + 8]);
    }

    #[test]
    fn a_line_with_no_end_past_the_limit_is_refused() {
        let mut lines = Lines::default();
        assert!(lines.push(&vec![b'x'; MAX_LINE + 1]).is_err());
    }

    #[test]
    fn stills_decode() {
        assert_eq!(
            still(&json!({ "data": "/9j/", "at": 1000.0 })),
            Some((vec![0xff, 0xd8, 0xff], Some(1000.0)))
        );
        assert_eq!(still(&json!({ "data": null })), None);
    }

    #[test]
    fn a_scripted_app_never_finds_the_real_browserbear() {
        let user = Path::new("/Users/me");
        assert_eq!(home(None, user, true), None);
        assert_eq!(
            home(None, user, false),
            Some(PathBuf::from(
                "/Users/me/Library/Application Support/BrowserBear"
            ))
        );
        assert_eq!(
            home(Some("/tmp/bear"), user, true),
            Some(PathBuf::from("/tmp/bear"))
        );
    }

    #[test]
    fn sanitize_url_strips_tokens_and_credentials_while_keeping_safe_params() {
        let (clean, removed) = sanitize_url(
            "https://alice:hunter2@app.example.com/items?page=3&author=mike&keyword=rust&token=1&access_token=2&id_token=3&refresh_token=4&auth=5&authorization=6&code=7&session=8&sid=9&key=10&api_key=11&x-api-key=12&password=13&pwd=14#access_token=frag",
        );
        assert!(removed);
        assert_eq!(
            clean,
            "https://app.example.com/items?page=3&author=mike&keyword=rust"
        );
        assert_eq!(
            sanitize_url("https://docs.example.com/a#intro"),
            ("https://docs.example.com/a#intro".to_owned(), false)
        );
        assert_eq!(sanitize_url("file:///etc/passwd"), (String::new(), true));
        assert_eq!(sanitize_url("javascript:alert(1)"), (String::new(), true));
        let huge = format!("https://example.com/x?q={}", "a".repeat(2500));
        assert_eq!(
            sanitize_url(&huge),
            ("https://example.com/x".to_owned(), true)
        );
    }

    #[test]
    fn pick_reply_enforces_capture_id_and_pinned_thread_and_resanitizes_url() {
        let raw = json!({
            "captureId": "pick-12345678",
            "state": "done",
            "capture": {
                "captureId": "pick-12345678",
                "thread": "thread-a",
                "kind": "element",
                "source": {
                    "title": "Dashboard",
                    "url": "https://app.example.com/dash?tab=main&access_token=secret123"
                },
                "screenshot": {
                    "data": "iVBORw0KGgo=",
                    "format": "png",
                    "width": 100,
                    "height": 40
                },
                "quote": "Selected line",
                "target": {
                    "tag": "button",
                    "role": "button",
                    "name": "Save",
                    "selector": "#save-btn",
                    "html": "<script>alert(1)</script>"
                },
                "warnings": ["closed_shadow_root"]
            }
        });
        assert!(parse_pick_reply(&raw, "pick-other", "thread-a").is_none());
        assert!(parse_pick_reply(&raw, "pick-12345678", "thread-b").is_none());
        let reply = parse_pick_reply(&raw, "pick-12345678", "thread-a").unwrap();
        assert_eq!(reply.state, PickState::Done);
        let cap = reply.capture.unwrap();
        assert_eq!(cap.source_url, "https://app.example.com/dash?tab=main");
        assert!(cap.warnings.iter().any(|w| w == "url_query_removed"));
        assert!(cap
            .warnings
            .iter()
            .any(|w| w == "may_contain_visible_secrets"));
        assert!(cap.warnings.iter().any(|w| w == "closed_shadow_root"));
        let block = cap.message_block();
        assert!(block.contains("From page: [Dashboard](https://app.example.com/dash?tab=main)"));
        assert!(block.contains("> Selected line"));
        assert!(!block.contains("<script>"));
    }

    #[test]
    fn capture_store_pins_to_thread_and_deduplicates_capture_id() {
        let mut store: CaptureStore<()> = CaptureStore::default();
        let cap = PickCapture {
            capture_id: "pick-dup-1".into(),
            thread: "thread-pinned".into(),
            created_at: None,
            kind: "text".into(),
            source_title: "Docs".into(),
            source_url: "https://example.com/docs".into(),
            screenshot: None,
            quote: Some("Important quote".into()),
            target: None,
            warnings: vec![],
        };
        assert!(store.insert(cap.clone(), ()));
        assert!(!store.insert(cap, ()), "duplicate capture_id ignored");
        assert_eq!(store.for_thread("thread-pinned").len(), 1);
        assert_eq!(store.for_thread("thread-focused-later").len(), 0);
        assert!(store.remove("thread-pinned", "pick-dup-1").is_some());
        assert_eq!(store.for_thread("thread-pinned").len(), 0);
    }
}
