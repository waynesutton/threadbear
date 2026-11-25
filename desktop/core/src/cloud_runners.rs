//! Settings › Runners' Cloud runners section and its Add a cloud runner
//! sheet: what the runner's `cloud.status` says (docs/protocol/keys.md,
//! "Cloud setup"), read into rows, and the words around them. The runner does
//! every step (runner/src/cloud-setup.ts); the app only shows them and starts
//! them, so `tb cloud` and the app always agree.

use serde_json::Value;

/// Behind the section's ⓘ.
pub const ABOUT: &str = "A machine per thread, started when you need one and asleep when it's idle. They show as Cloud in the composer's Runs on menu, never as machines above. Threadbear sets one up on Fly from one token: the app for the boxes, the box image and the keys.";
/// The section's Ask a thread line.
pub const ASK: &str = "explain how cloud runners work, or what a step that failed means. Setting one up is yours to do here, or with tb cloud setup in a terminal.";
/// The sheet's line under its title.
pub const SHEET_ABOUT: &str =
    "Threadbear makes the Fly app, the box image and the keys. You give it one token.";
pub const EMPTY: &str = "No cloud runner yet";
pub const ADD: &str = "Add a cloud runner…";
pub const ADD_TIP: &str = "Set up a cloud runner on Fly from one token";
pub const TOKEN_TIP: &str =
    "An org token from fly tokens create org, or fly.io/tokens. It goes only to your backend";
pub const FLYCTL: &str = "Set up with flyctl's sign-in";
pub const FLYCTL_TIP: &str =
    "Start now, with a token flyctl makes on this Mac. It must be signed in: fly auth login";
/// The sheet's Before you start: what to run in a terminal first, with flyctl
/// not yet on this Mac or not signed in.
pub const BEFORE: &str = "Before you start, in Terminal:";
pub const BEFORE_STEPS: [(&str, &str); 2] = [
    (
        "brew install flyctl",
        "installs flyctl, Fly's command line tool",
    ),
    (
        "fly auth login",
        "signs it in to your Fly account in the browser",
    ),
];
pub const BEFORE_AFTER: &str = "Then press Set up with flyctl's sign-in. Or paste an org token from fly tokens create org personal (or fly.io/dashboard/personal/tokens) and press Add.";
pub const CHECK: &str = "Check again";
pub const CHECK_TIP: &str = "Check the token, the app, the backend's keys and the runner again";
pub const REBUILD: &str = "Rebuild";
pub const REBUILD_TIP: &str = "Build the box image again from this Mac's copy of Threadbear";
pub const REMOVE: &str = "Remove…";
pub const REMOVE_TIP: &str = "Take this cloud runner out of Threadbear";
pub const REMOVE_ABOUT: &str = "Threadbear stops offering it in Runs on and clears the Fly token and app from your backend. Its threads must be archived first. Your seal key stays, so other cloud logins keep working.";
pub const REMOVE_GO: &str = "Remove";
pub const REMOVE_GO_TIP: &str = "Remove it now. You can add it again later";
pub const REMOVE_KEEP_APP: &str = "The Fly app stays on Fly with nothing running, so adding it again is quick. Delete it any time with fly apps destroy.";
pub const EDIT: &str = "Edit…";
pub const EDIT_TIP: &str = "Change the name or box size, or give it a new token";
/// What a runner too old for `cloud.*` gets, in place of the section.
pub const TOO_OLD: &str =
    "This Mac's runner is too old to set up cloud runners. It updates by itself.";

/// The box sizes offered: (CPUs, MB). Fly can pause a box of 2 GB or less.
pub const SIZES: [(u32, u32); 3] = [(1, 1024), (1, 2048), (2, 2048)];
pub const DEFAULT_SIZE: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Todo,
    Doing,
    Ok,
    Warn,
    Fail,
    Skipped,
}

impl StepState {
    fn parse(s: &str) -> Self {
        match s {
            "doing" => Self::Doing,
            "ok" => Self::Ok,
            "warn" => Self::Warn,
            "fail" => Self::Fail,
            "skipped" => Self::Skipped,
            _ => Self::Todo,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Doing => "doing",
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub state: StepState,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub kind: String,
    pub running: bool,
    pub steps: Vec<Step>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub image: String,
    pub cpus: u32,
    pub memory_mb: u32,
    pub running: u32,
    pub asleep: u32,
}

/// `cloud.cost`: this month's estimate, from Fly's start and stop times.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cost {
    pub cents: u64,
    /// Hours the boxes ran, in tenths.
    pub tenth_hours: u64,
    pub boxes: u32,
    pub as_of_ms: i64,
}

impl Cost {
    pub fn decode(v: &Value) -> Option<Self> {
        let usd = v["usd"].as_f64()?;
        Some(Cost {
            cents: (usd.max(0.0) * 100.0).round() as u64,
            tenth_hours: (v["hours"].as_f64().unwrap_or(0.0).max(0.0) * 10.0).round() as u64,
            boxes: n(v, "boxes"),
            as_of_ms: v["asOf"].as_i64().unwrap_or(0),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub job: Option<Job>,
    pub services: Vec<Service>,
    pub cost: Option<Cost>,
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().to_owned()
}

fn n(v: &Value, k: &str) -> u32 {
    v[k].as_u64()
        .and_then(|x| u32::try_from(x).ok())
        .unwrap_or_default()
}

impl Status {
    /// `cloud.status`'s answer. Unknown fields are skipped; a step with no id, dropped.
    pub fn decode(v: &Value) -> Self {
        let job = v.get("job").filter(|j| j.is_object()).map(|j| Job {
            kind: s(j, "kind"),
            running: j["running"].as_bool().unwrap_or(false),
            steps: j["steps"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter(|x| x["id"].is_string())
                        .map(|x| Step {
                            id: s(x, "id"),
                            title: s(x, "title"),
                            state: StepState::parse(x["state"].as_str().unwrap_or("")),
                            detail: x["detail"].as_str().map(str::to_owned),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            error: j["error"].as_str().map(str::to_owned),
        });
        let services = v["services"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|x| x["serviceId"].is_string())
                    .map(|x| Service {
                        id: s(x, "serviceId"),
                        name: s(x, "name"),
                        provider: s(x, "provider"),
                        image: s(x, "image"),
                        cpus: n(&x["box"], "cpus"),
                        memory_mb: n(&x["box"], "memoryMb"),
                        running: n(x, "running"),
                        asleep: n(x, "asleep"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let cost = v.get("cost").and_then(Cost::decode);
        Status {
            job,
            services,
            cost,
        }
    }

    pub fn running(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.running)
    }

    /// The last job finished with every step through (ok, warn or skipped by a check).
    pub fn done_ok(&self, kind: &str) -> bool {
        self.job
            .as_ref()
            .is_some_and(|j| j.kind == kind && !j.running && j.error.is_none())
    }
}

/// The remove dialog's title: "Remove Cloud (Fly)?".
pub fn remove_title(name: &str) -> String {
    format!("Remove {name}?")
}

/// The checkbox: "Also delete the Fly app tb-boxes-x, with its boxes and image".
pub fn remove_app_label(app: &str) -> String {
    format!("Also delete the Fly app {app}, with its boxes and image")
}

/// The Fly app a runner's image lives in: registry.fly.io/<app>:<tag>.
pub fn fly_app(image: &str) -> Option<String> {
    let rest = image.strip_prefix("registry.fly.io/")?;
    let app = rest.split([':', '@']).next()?;
    (!app.is_empty()).then(|| app.to_owned())
}

/// "2 GB", "1.5 GB".
pub fn memory_label(mb: u32) -> String {
    if mb.is_multiple_of(1024) {
        format!("{} GB", mb / 1024)
    } else {
        format!("{:.1} GB", f64::from(mb) / 1024.0)
    }
}

/// "1 CPU, 2 GB".
pub fn size_label(cpus: u32, mb: u32) -> String {
    format!(
        "{cpus} CPU{}, {}",
        if cpus == 1 { "" } else { "s" },
        memory_label(mb)
    )
}

/// Beside the name: "Fly · 1 CPU, 2 GB".
pub fn service_line(svc: &Service) -> String {
    let provider = match svc.provider.as_str() {
        "fly" => "Fly",
        other => other,
    };
    format!("{provider} · {}", size_label(svc.cpus, svc.memory_mb))
}

/// "2 running · 6 asleep", or "None running" with none of either.
pub fn threads_line(svc: &Service) -> String {
    if svc.running == 0 && svc.asleep == 0 {
        "None running".to_owned()
    } else {
        format!("{} running · {} asleep", svc.running, svc.asleep)
    }
}

pub const COST_LABEL: &str = "This month";
pub const COST_TIP: &str = "An estimate at Fly's list price, from each box's start and stop times this month. Fly has no billing API, so your bill is at fly.io/dashboard. Fly is asked only when Settings opens or you press refresh, never by an agent";
pub const COST_REFRESH_TIP: &str = "Ask Fly again";

/// "$1.23", "Under $0.01" or "$0.00".
pub fn cost_amount(c: &Cost) -> String {
    if c.cents == 0 && c.tenth_hours > 0 {
        "Under $0.01".to_owned()
    } else {
        format!("${}.{:02}", c.cents / 100, c.cents % 100)
    }
}

/// "3.5 hours on 4 boxes", "No boxes yet".
pub fn cost_detail(c: &Cost) -> String {
    if c.boxes == 0 {
        return "No boxes ran yet".to_owned();
    }
    let hours = if c.tenth_hours < 1 {
        "under 0.1 hours".to_owned()
    } else if c.tenth_hours == 10 {
        "1 hour".to_owned()
    } else {
        format!("{}.{} hours", c.tenth_hours / 10, c.tenth_hours % 10)
    };
    format!(
        "{hours} on {} box{}",
        c.boxes,
        if c.boxes == 1 { "" } else { "es" }
    )
}

/// The badge by the name: what the last job here says of it.
pub fn badge(status: &Status) -> Option<(&'static str, StepState)> {
    let job = status.job.as_ref()?;
    if job.running {
        return Some(("Checking", StepState::Doing));
    }
    if job.steps.iter().any(|s| s.state == StepState::Fail) {
        Some(("Needs a look", StepState::Fail))
    } else {
        Some(("Ready", StepState::Ok))
    }
}

/// What the sheet's main button says: Add, then Done once set up, or Try again.
pub fn sheet_button(status: Option<&Status>, started: bool, editing: bool) -> &'static str {
    let idle = if editing { "Save" } else { "Add" };
    if !started {
        return idle;
    }
    match status.and_then(|s| s.job.as_ref()) {
        Some(j) if j.kind == "setup" && j.running => idle,
        Some(j) if j.kind == "setup" && j.error.is_none() => "Done",
        Some(j) if j.kind == "setup" => "Try again",
        _ => idle,
    }
}

/// A dump line for a step: "ok Token works: org personal".
pub fn step_line(step: &Step) -> String {
    match &step.detail {
        Some(d) => format!("{} {}: {d}", step.state.name(), step.title),
        None => format!("{} {}", step.state.name(), step.title),
    }
}

/// Whether a folder can start a cloud thread, before send: a box clones the
/// folder's repo from its origin, so it must be a git repo whose origin a box
/// can reach (runner/src/virtual-runners.ts repoOf and boxOrigin). The runner
/// still refuses the rest; this only says so before Send is pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderCheck {
    Checking,
    Fine,
    /// Why not, for the line under the box and Send's tip.
    Not(String),
}

/// An origin a box clones over https: an https address, or GitHub's ssh forms.
pub fn box_can_clone(origin: &str) -> bool {
    let s = origin.trim();
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return s.len() > "https://".len();
    }
    let path = lower
        .strip_prefix("ssh://git@github.com/")
        .or_else(|| lower.strip_prefix("ssh://github.com/"))
        .or_else(|| {
            (!lower.contains("://"))
                .then(|| {
                    lower
                        .strip_prefix("git@github.com:")
                        .or_else(|| lower.strip_prefix("github.com:"))
                })
                .flatten()
        });
    path.is_some_and(|p| {
        let mut parts = p.trim_end_matches('/').split('/');
        matches!((parts.next(), parts.next(), parts.next()), (Some(o), Some(r), None) if !o.is_empty() && !r.is_empty())
    })
}

/// What `git rev-parse --show-toplevel` and `git remote get-url origin` said
/// (None when they failed), as the check for a cloud runner named `name`.
/// `folder` is the folder's path, named in the words by its last part;
/// `inside` the git repos directly inside it, for one that isn't a repo.
pub fn folder_check(
    name: &str,
    folder: &str,
    top: Option<&str>,
    origin: Option<&str>,
    inside: &[String],
) -> FolderCheck {
    let top = top.map(str::trim).filter(|t| !t.is_empty());
    let origin = origin.map(str::trim).filter(|o| !o.is_empty());
    let folder = folder_name(folder);
    match (top, origin) {
        (None, _) => FolderCheck::Not(not_a_repo(name, &folder, inside)),
        (Some(_), None) => FolderCheck::Not(format!(
            "{folder} isn't on GitHub yet, so {name} can't copy it. Push it to GitHub first."
        )),
        (Some(_), Some(o)) if !box_can_clone(o) => FolderCheck::Not(format!(
            "{name} can't reach {folder}'s origin. It copies repos from GitHub or an https address."
        )),
        _ => FolderCheck::Fine,
    }
}

/// A folder's name for the warning: its path's last part.
fn folder_name(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches('/');
    match trimmed.rsplit('/').next().filter(|n| !n.is_empty()) {
        Some(n) => n.to_owned(),
        None => "This folder".to_owned(),
    }
}

/// The usual case: a folder that isn't a git repo. The + menu in the
/// message box is the one way to another folder, so the words point there;
/// a repo found directly inside is named, as that's the one usually meant.
pub fn not_a_repo(name: &str, folder: &str, inside: &[String]) -> String {
    match inside {
        [] => format!(
            "{folder} isn't a git repo, so {name} can't copy it. Press + to pick a repo's folder."
        ),
        [one] => format!(
            "{folder} isn't a git repo, but {one} inside it is. Press + and pick that folder."
        ),
        [a, b, ..] => format!(
            "{folder} isn't a git repo, but {a} and {b} inside it are. Press + and pick one of them."
        ),
    }
}

/// How many repos inside a folder the warning names at most.
pub const INSIDE_MAX: usize = 2;

/// Short, beside the runner in Runs on.
pub const MENU_NOT_HERE: &str = "needs a GitHub repo folder";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "job": { "kind": "setup", "running": false, "startedAt": 1, "steps": [
                { "id": "flyctl", "title": "flyctl installed", "state": "ok", "detail": "v0.3.230" },
                { "id": "updates", "title": "Image follows main (GitHub)", "state": "warn", "detail": "gh isn't signed in" },
                { "id": "test", "title": "Test box", "state": "ok", "extra": 1 },
                { "title": "no id" }
            ]},
            "services": [
                { "serviceId": "s1", "name": "Cloud (Fly)", "provider": "fly", "image": "registry.fly.io/x:latest", "box": { "cpus": 1, "memoryMb": 2048 }, "running": 2, "asleep": 6 },
                { "name": "no id" }
            ],
            "later": true
        })
    }

    #[test]
    fn decodes_status_and_skips_what_it_cannot_read() {
        let st = Status::decode(&sample());
        let job = st.job.clone().unwrap();
        assert_eq!(job.steps.len(), 3);
        assert_eq!(job.steps[1].state, StepState::Warn);
        assert_eq!(st.services.len(), 1);
        assert_eq!(service_line(&st.services[0]), "Fly · 1 CPU, 2 GB");
        assert_eq!(threads_line(&st.services[0]), "2 running · 6 asleep");
        assert_eq!(badge(&st), Some(("Ready", StepState::Ok)));
        assert!(st.done_ok("setup"));
        assert_eq!(step_line(&job.steps[0]), "ok flyctl installed: v0.3.230");
    }

    #[test]
    fn empty_and_running() {
        let st = Status::decode(&json!({ "services": [] }));
        assert_eq!(st, Status::default());
        assert_eq!(badge(&st), None);
        let going = Status::decode(
            &json!({ "job": { "kind": "check", "running": true, "steps": [] }, "services": [] }),
        );
        assert!(going.running());
        assert_eq!(badge(&going), Some(("Checking", StepState::Doing)));
    }

    #[test]
    fn sheet_button_follows_the_setup() {
        let failed = Status::decode(
            &json!({ "job": { "kind": "setup", "running": false, "error": "x", "steps": [] } }),
        );
        let running =
            Status::decode(&json!({ "job": { "kind": "setup", "running": true, "steps": [] } }));
        let ok =
            Status::decode(&json!({ "job": { "kind": "setup", "running": false, "steps": [] } }));
        assert_eq!(sheet_button(None, false, false), "Add");
        assert_eq!(sheet_button(Some(&running), true, true), "Save");
        assert_eq!(sheet_button(Some(&ok), true, false), "Done");
        assert_eq!(sheet_button(Some(&failed), true, false), "Try again");
        // A check left from before the sheet opened isn't this setup's.
        let check =
            Status::decode(&json!({ "job": { "kind": "check", "running": false, "steps": [] } }));
        assert_eq!(sheet_button(Some(&check), false, false), "Add");
    }

    #[test]
    fn cost_lines() {
        let st = Status::decode(
            &json!({ "services": [], "cost": { "usd": 1.234, "hours": 3.46, "boxes": 4, "asOf": 5 } }),
        );
        let c = st.cost.unwrap();
        assert_eq!(cost_amount(&c), "$1.23");
        assert_eq!(cost_detail(&c), "3.5 hours on 4 boxes");
        let tiny = Cost::decode(&json!({ "usd": 0, "hours": 0, "boxes": 2 })).unwrap();
        assert_eq!(cost_amount(&tiny), "$0.00");
        assert_eq!(cost_detail(&tiny), "under 0.1 hours on 2 boxes");
        let some = Cost::decode(&json!({ "usd": 0.001, "hours": 0.5, "boxes": 1 })).unwrap();
        assert_eq!(cost_amount(&some), "Under $0.01");
        assert_eq!(cost_detail(&some), "0.5 hours on 1 box");
        assert_eq!(Status::decode(&json!({ "services": [] })).cost, None);
    }

    #[test]
    fn remove_words() {
        assert_eq!(
            fly_app("registry.fly.io/tb-boxes-x:latest").as_deref(),
            Some("tb-boxes-x")
        );
        assert_eq!(fly_app("ghcr.io/x/y:latest"), None);
        assert_eq!(remove_title("Cloud (Fly)"), "Remove Cloud (Fly)?");
        assert!(remove_app_label("tb-boxes-x").contains("tb-boxes-x"));
        assert!(!REMOVE_ABOUT.contains('\u{2014}'));
    }

    #[test]
    fn sizes() {
        assert_eq!(size_label(2, 2048), "2 CPUs, 2 GB");
        assert_eq!(memory_label(1536), "1.5 GB");
        assert!(SIZES.iter().all(|(_, mb)| *mb <= 2048 && mb % 256 == 0));
    }

    #[test]
    fn folder_checks() {
        let n = "Cloud (Fly)";
        let f = "/Users/x/sites/waynesuttonai";
        let check = |top, origin| folder_check(n, f, top, origin, &[]);
        assert_eq!(
            check(None, None),
            FolderCheck::Not(
                "waynesuttonai isn't a git repo, so Cloud (Fly) can't copy it. Press + to pick a repo's folder."
                    .to_owned()
            )
        );
        assert!(
            matches!(check(Some("/r"), None), FolderCheck::Not(w) if w.starts_with("waynesuttonai isn't on GitHub yet"))
        );
        assert!(
            matches!(check(Some("/r"), Some("/Users/x/repo")), FolderCheck::Not(w) if w.contains("waynesuttonai's origin"))
        );
        assert_eq!(
            check(Some("/r"), Some("https://github.com/o/r.git\n")),
            FolderCheck::Fine
        );
        assert_eq!(
            check(Some("/r"), Some("git@github.com:o/r.git")),
            FolderCheck::Fine
        );
        assert_eq!(
            check(Some("/r"), Some("ssh://git@github.com/o/r")),
            FolderCheck::Fine
        );
        assert!(!box_can_clone("git@gitlab.com:o/r.git"));
        assert!(!box_can_clone("http://github.com/o/r"));
        assert!(!box_can_clone("file:///x"));
        assert!(!box_can_clone("git@github.com:o"));
    }

    #[test]
    fn the_warning_names_the_folder_a_repo_inside_it_and_the_plus_menu() {
        let n = "Cloud (Fly)";
        let one = ["waynesutton-ai".to_owned()];
        assert_eq!(
            folder_check(n, "/Users/x/sites/waynesuttonai/", None, None, &one),
            FolderCheck::Not(
                "waynesuttonai isn't a git repo, but waynesutton-ai inside it is. Press + and pick that folder."
                    .to_owned()
            )
        );
        let two = ["a".to_owned(), "b".to_owned()];
        assert_eq!(
            not_a_repo(n, "sites", &two),
            "sites isn't a git repo, but a and b inside it are. Press + and pick one of them."
        );
        assert!(not_a_repo(n, &folder_name("/"), &[]).starts_with("This folder isn't a git repo"));
        // One way out, the + menu: every not-a-repo wording points there.
        for w in [
            not_a_repo(n, "x", &[]),
            not_a_repo(n, "x", &one),
            not_a_repo(n, "x", &two),
        ] {
            assert!(w.contains("Press +"));
            assert!(!w.contains('\u{2014}'));
        }
        assert!(!MENU_NOT_HERE.contains('\u{2014}'));
    }
}
