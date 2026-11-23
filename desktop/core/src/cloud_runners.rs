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
pub const FLYCTL: &str = "Use flyctl's sign-in";
pub const FLYCTL_TIP: &str =
    "Make a token with flyctl on this Mac, which must be signed in (fly auth login)";
pub const CHECK: &str = "Check again";
pub const CHECK_TIP: &str = "Check the token, the app, the backend's keys and the runner again";
pub const REBUILD: &str = "Rebuild";
pub const REBUILD_TIP: &str = "Build the box image again from this Mac's copy of Threadbear";
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub job: Option<Job>,
    pub services: Vec<Service>,
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
        Status { job, services }
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
    fn sizes() {
        assert_eq!(size_label(2, 2048), "2 CPUs, 2 GB");
        assert_eq!(memory_label(1536), "1.5 GB");
        assert!(SIZES.iter().all(|(_, mb)| *mb <= 2048 && mb % 256 == 0));
    }
}
