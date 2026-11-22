// Mike's Connectors gateway, as the runner last heard from its /health
// (docs/protocol.md, "Connectors"): `connectors.status` and
// `connectors.updated`, and what the Connectors panel and its activity bar
// icon say about it, a port of Connectors.swift and ConnectorsText.swift.
// The accent dot only when a service is signed out, as only Mike can sign it
// back in; a grey one while the gateway can't be reached or a service is
// connecting or in error, which usually sorts itself out (as Updates' dots
// go, 27 Oct). The app never calls the gateway itself: it only opens its
// pages in the browser.
use crate::inbox::parse_ms;
use serde_json::Value;
use std::collections::BTreeSet;

/// `ConnectorsStatus`, read leniently, as the gateway's answer is still
/// changing: an unknown state is shown as it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectorsStatus {
    /// ok, unreachable, off or checking.
    pub state: String,
    /// The gateway's base, https://host:port.
    pub url: Option<String>,
    pub checked_ms: Option<i64>,
    pub error: Option<String>,
    pub version: Option<String>,
    pub services: Vec<ConnectorService>,
    pub can_set_up: bool,
    pub can_add_local: bool,
    pub catalog: Vec<ConnectorCatalogEntry>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectorService {
    pub id: String,
    pub title: String,
    /// connected, signed_out, connecting, error, disabled or unknown (or a
    /// newer word).
    pub state: String,
    pub message: Option<String>,
    /// Where Mike signs it in; set on anything not connected or disabled.
    pub connect_url: Option<String>,
    pub local: bool,
    pub state_note: Option<String>,
    /// It needs an OAuth app of the user's own that the gateway hasn't got
    /// yet (Slack): the Create button and paste form instead of Connect.
    pub client_setup: Option<ClientSetup>,
}

/// `clientSetup` (runner static-clients.ts): the link that makes the app,
/// filled in, the redirect it was filled in with, and a line of help.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientSetup {
    pub create_url: String,
    pub redirect_url: String,
    pub help: String,
}

impl ClientSetup {
    fn decode(v: &Value) -> Option<ClientSetup> {
        let create_url = v["createUrl"]
            .as_str()
            .filter(|u| u.starts_with("https://"))?
            .to_owned();
        Some(ClientSetup {
            create_url,
            redirect_url: v["redirectUrl"].as_str().unwrap_or_default().to_owned(),
            help: v["help"].as_str().unwrap_or_default().to_owned(),
        })
    }
}

impl ConnectorsStatus {
    /// `{ connectors }`' value.
    pub fn decode(v: &Value) -> ConnectorsStatus {
        let text = |key: &str| v[key].as_str().map(str::to_owned);
        ConnectorsStatus {
            state: v["state"].as_str().unwrap_or("checking").to_owned(),
            url: text("url"),
            checked_ms: v["checkedAt"].as_str().and_then(parse_ms),
            error: text("error"),
            version: text("version"),
            can_set_up: v["canSetUp"].as_bool().unwrap_or(false),
            can_add_local: v["canAddLocal"].as_bool().unwrap_or(false),
            catalog: v["catalog"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(ConnectorCatalogEntry::decode)
                .collect(),
            services: v["services"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(ConnectorService::decode)
                .collect(),
        }
    }
}

impl ConnectorService {
    fn decode(v: &Value) -> Option<ConnectorService> {
        let id = v["id"].as_str()?.to_owned();
        Some(ConnectorService {
            title: v["title"]
                .as_str()
                .map_or_else(|| id.clone(), str::to_owned),
            state: v["state"].as_str().unwrap_or("unknown").to_owned(),
            message: v["message"].as_str().map(str::to_owned),
            connect_url: v["connectUrl"].as_str().map(str::to_owned),
            local: v["local"].as_bool().unwrap_or(false),
            state_note: v["stateNote"].as_str().map(str::to_owned),
            client_setup: ClientSetup::decode(&v["clientSetup"]),
            id,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectorCatalogEntry {
    pub id: String,
    pub title: String,
    pub blurb: String,
    pub setup_note: Option<String>,
    pub kind: String,
    pub command: Option<String>,
    pub asks: Vec<String>,
    pub client_setup: Option<ClientSetup>,
}

impl ConnectorCatalogEntry {
    pub fn decode(v: &Value) -> Option<Self> {
        let id = v["id"].as_str()?.to_owned();
        Some(Self {
            title: v["title"].as_str().unwrap_or(&id).to_owned(),
            id,
            blurb: v["blurb"].as_str().unwrap_or_default().to_owned(),
            setup_note: v["setupNote"].as_str().map(str::to_owned),
            kind: v["kind"].as_str().unwrap_or("remote").to_owned(),
            command: v["command"].as_str().map(str::to_owned),
            asks: v["asks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            client_setup: ClientSetup::decode(&v["clientSetup"]),
        })
    }
    pub fn is_local(&self) -> bool {
        self.kind == "local"
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentKey {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub read_all: bool,
    pub expires_at: String,
    pub last_used_ms: Option<i64>,
    pub expired: bool,
}

impl AgentKey {
    pub fn decode(v: &Value) -> Option<Self> {
        Some(Self {
            id: v["id"].as_str()?.to_owned(),
            name: v["name"].as_str().unwrap_or("Agent").to_owned(),
            scope: v["scope"].as_str().unwrap_or("read").to_owned(),
            read_all: v["readAll"].as_bool().unwrap_or(false),
            expires_at: v["expiresAt"].as_str().unwrap_or_default().to_owned(),
            last_used_ms: v["lastUsedAt"].as_str().and_then(parse_ms),
            expired: v["expired"].as_bool().unwrap_or(false),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentsList {
    pub on: bool,
    pub reason: Option<String>,
    pub url: Option<String>,
    pub keys: Vec<AgentKey>,
}
impl AgentsList {
    pub fn decode(v: &Value) -> Self {
        Self {
            on: v["on"].as_bool().unwrap_or(false),
            reason: v["reason"].as_str().map(str::to_owned),
            url: v["url"].as_str().map(str::to_owned),
            keys: v["keys"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(AgentKey::decode)
                .collect(),
        }
    }
}

// Deliberately no Debug or serialization: the secret stays in the open panel.
pub struct AgentConnected {
    pub key: AgentKey,
    pub secret: String,
    pub url: String,
}
impl AgentConnected {
    pub fn decode(v: &Value) -> Option<Self> {
        Some(Self {
            key: AgentKey::decode(&v["key"])?,
            secret: v["secret"].as_str()?.to_owned(),
            url: v["url"].as_str()?.to_owned(),
        })
    }
}

pub fn shows_icon(status: Option<&ConnectorsStatus>, agents: Option<&AgentsList>) -> bool {
    status.is_some_and(|s| s.state != "off" || s.can_set_up) || agents.is_some_and(|a| a.on)
}
pub fn scope_line(key: &AgentKey) -> String {
    format!(
        "{}, {}",
        if key.scope == "write" {
            "Starts Ask threads"
        } else {
            "Read only"
        },
        if key.read_all {
            "reads all your threads"
        } else {
            "reads threads it started"
        }
    )
}
pub fn last_used(key: &AgentKey, now_ms: i64) -> String {
    if key.expired {
        return "Expired".into();
    }
    let Some(at) = key.last_used_ms else {
        return "Not used yet".into();
    };
    let mins = (now_ms - at) / 60_000;
    if mins < 2 {
        "Used just now".into()
    } else if mins < 60 {
        format!("Used {mins} minutes ago")
    } else if mins < 2880 {
        format!("Used {} hours ago", mins / 60)
    } else {
        format!("Used {} days ago", mins / 1440)
    }
}

pub struct AgentFormField {
    pub label: &'static str,
    pub value: String,
    pub shown: String,
    pub copies: bool,
    pub tip: &'static str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OutsideAgentApp {
    #[default]
    Grok,
    Notion,
    Slack,
}
impl OutsideAgentApp {
    pub const ALL: [Self; 3] = [Self::Grok, Self::Notion, Self::Slack];
    pub fn name(self) -> &'static str {
        match self {
            Self::Grok => "Grok Bot",
            Self::Notion => "Notion",
            Self::Slack => "Slack",
        }
    }
    pub fn can_connect(self) -> bool {
        self != Self::Slack
    }
    pub fn for_key(name: &str) -> Self {
        if name == "Notion" {
            Self::Notion
        } else {
            Self::Grok
        }
    }
    pub fn needs(self) -> &'static str {
        match self {
            Self::Grok => "xAI's agent in the cloud. On a team plan, your admin's connector policy has to allow servers by address.",
            Self::Notion => "For Notion's Custom Agents, on the Business or Enterprise plan. A workspace admin turns on custom MCP servers first, in Settings > Connections.",
            Self::Slack => "Slackbot only connects to servers that sign in with OAuth or with Slack's own sign-in, through a Slack app. It can't take a key, so Threadbear can't connect to it yet.",
        }
    }
    pub fn guide(self) -> &'static str {
        match self {
            Self::Grok => "https://docs.x.ai/grok-bot/computer-and-apps",
            Self::Notion => "https://www.notion.com/help/mcp-connections-for-custom-agents",
            Self::Slack => "https://docs.slack.dev/ai/slackbot-mcp-client/",
        }
    }
    pub fn intro(self) -> &'static str {
        match self {
            Self::Grok => "In Grok Bot, open Connect apps, then Add a custom connector, and fill in each box:",
            Self::Notion => "In your Custom Agent's settings, open Tools & Access, then Add connection, then Custom MCP server, and fill in each box:",
            Self::Slack => "",
        }
    }
    pub fn outro(self) -> &'static str {
        match self {
            Self::Grok => "Save it, then ask Grok Bot to list your Threadbear threads. Its row here says Used just now once it works. If the key ever leaks, Revoke it and make a new one.",
            Self::Notion => "Connect it, then ask the agent to list your Threadbear threads. Its row here says Used just now once it works. If the key ever leaks, Revoke it and make a new one.",
            Self::Slack => "",
        }
    }
    pub fn form(self, url: &str, secret: &str) -> Vec<AgentFormField> {
        if !self.can_connect() {
            return vec![];
        }
        let field = |label, value: &str, copies, tip| AgentFormField {
            label,
            value: value.into(),
            shown: value.into(),
            copies,
            tip,
        };
        let mut fields = vec![field(
            "Name",
            if self == Self::Notion {
                "Threadbear"
            } else {
                "threadbear"
            },
            true,
            "Copy a name for the connector",
        )];
        if self == Self::Grok {
            fields.push(field(
                "Type",
                "Remote HTTPS",
                false,
                "Pick Remote HTTPS in the menu, not Command",
            ));
        }
        fields.push(field("Server URL", url, true, "Copy the server address"));
        if self == Self::Notion {
            fields.push(field(
                "Auth",
                "A header",
                false,
                "Pick header-based auth, not OAuth",
            ));
        }
        fields.push(field(
            "Header key",
            "Authorization",
            true,
            "Copy the header name, for the left box under HTTP headers",
        ));
        fields.push(AgentFormField {
            label: "Header value",
            value: format!("Bearer {secret}"),
            shown: format!(
                "Bearer {}••••••••",
                if secret.starts_with("tba_") {
                    "tba_"
                } else {
                    ""
                }
            ),
            copies: true,
            tip: "Copy \"Bearer\", a space and the key, for the right box under HTTP headers",
        });
        if self == Self::Grok {
            fields.push(field(
                "Client ID",
                "Leave it empty",
                false,
                "Only for services you sign in to in a browser; the key does that here",
            ));
        }
        fields
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalServer {
    pub launcher: String,
    pub package: String,
    pub version: String,
    pub title: String,
}
impl LocalServer {
    pub fn new(launcher: &str, package: &str, version: &str, title: &str) -> Self {
        Self {
            launcher: launcher.into(),
            package: package.trim().into(),
            version: version.trim().into(),
            title: title.trim().into(),
        }
    }
    pub fn command(&self) -> String {
        format!(
            "{} {}{}@{}",
            self.launcher,
            if self.launcher == "npx" { "-y " } else { "" },
            self.package,
            self.version
        )
    }
    pub fn key(&self) -> String {
        format!("local:{}:{}@{}", self.launcher, self.package, self.version)
    }
    // Matches Swift's exact-version grammar without adding a regex dependency.
    pub fn is_pinned(&self) -> bool {
        let mut package = self.package.bytes();
        if !package
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'@')
            || !package.all(|b| b.is_ascii_alphanumeric() || b"._/-".contains(&b))
        {
            return false;
        }
        fn digits(s: &mut &str, max: usize) -> bool {
            let n = s.bytes().take_while(u8::is_ascii_digit).count();
            if n == 0 || n > max {
                return false;
            }
            *s = &s[n..];
            true
        }
        let mut version = self.version.as_str();
        if !digits(&mut version, 6) {
            return false;
        }
        match self.launcher.as_str() {
            "npx" => {
                for _ in 0..2 {
                    let Some(rest) = version.strip_prefix('.') else {
                        return false;
                    };
                    version = rest;
                    if !digits(&mut version, 6) {
                        return false;
                    }
                }
                version.is_empty()
                    || version.strip_prefix('-').is_some_and(|rest| {
                        rest.split('.').all(|part| {
                            !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric())
                        })
                    })
            }
            "uvx" => {
                for _ in 0..3 {
                    if !version.starts_with('.')
                        || !version.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
                    {
                        break;
                    }
                    version = &version[1..];
                    if !digits(&mut version, 6) {
                        return false;
                    }
                }
                for prefix in ["rc", "a", "b"] {
                    if let Some(rest) = version.strip_prefix(prefix) {
                        version = rest;
                        if !digits(&mut version, 4) {
                            return false;
                        }
                        break;
                    }
                }
                for prefix in [".post", ".dev"] {
                    if let Some(rest) = version.strip_prefix(prefix) {
                        version = rest;
                        if !digits(&mut version, 4) {
                            return false;
                        }
                    }
                }
                version.is_empty()
            }
            _ => false,
        }
    }
}

pub fn trust_lines(asks: &[String]) -> Vec<String> {
    vec!["Runs on this Mac whenever Connectors is running, as you, with what you can reach. Every login and Codex gets its tools.".into(),
        if asks.is_empty() { "None of its tools ask first, unless the gateway's own rules catch them (sends, posts and deletes).".into() }
        else { format!("These wait for your OK on the gateway's approval page: {}.", list(&asks.iter().map(String::as_str).collect::<Vec<_>>())) },
        "It stays at this version until you add a newer one.".into()]
}

pub const HIDDEN_AGENT: &str = "agent:grok-bot";
pub fn hidden_decode(raw: &str) -> BTreeSet<String> {
    raw.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
pub fn hidden_encode(hidden: &BTreeSet<String>) -> String {
    hidden.iter().cloned().collect::<Vec<_>>().join("\n")
}
pub fn visible_status(
    status: Option<&ConnectorsStatus>,
    hidden: &BTreeSet<String>,
) -> Option<ConnectorsStatus> {
    status.cloned().map(|mut s| {
        s.services
            .retain(|r| !hidden.contains(&format!("service:{}", r.id)));
        s.catalog
            .retain(|r| !hidden.contains(&format!("catalog:{}", r.id)));
        s
    })
}
pub fn show_agents(agents: Option<&AgentsList>, hidden: &BTreeSet<String>) -> bool {
    agents.is_some_and(|a| a.on && (!hidden.contains(HIDDEN_AGENT) || !a.keys.is_empty()))
}
pub fn hidden_items(
    hidden: &BTreeSet<String>,
    status: Option<&ConnectorsStatus>,
    agents: Option<&AgentsList>,
) -> Vec<(String, String)> {
    let mut rows = vec![];
    if let Some(s) = status {
        rows.extend(
            s.services
                .iter()
                .map(|r| (format!("service:{}", r.id), r.title.clone())),
        );
        if s.state == "ok" {
            rows.extend(
                s.catalog
                    .iter()
                    .map(|r| (format!("catalog:{}", r.id), r.title.clone())),
            );
        }
    }
    if agents.is_some_and(|a| a.on && a.reason.is_none()) {
        rows.push((HIDDEN_AGENT.into(), "Connect an agent".into()));
    }
    rows.retain(|(id, _)| hidden.contains(id));
    rows
}

/// The services that need Mike to sign in again.
pub fn signed_out(s: Option<&ConnectorsStatus>) -> Vec<&ConnectorService> {
    s.filter(|s| s.state == "ok").map_or_else(Vec::new, |s| {
        s.services
            .iter()
            .filter(|x| x.state == "signed_out")
            .collect()
    })
}

/// The accent dot.
pub fn needs_you(s: Option<&ConnectorsStatus>) -> bool {
    !signed_out(s).is_empty()
}

/// The grey dot: something's off that isn't Mike's to fix yet.
pub fn quiet(s: Option<&ConnectorsStatus>) -> bool {
    let Some(s) = s else { return false };
    match s.state.as_str() {
        "unreachable" => true,
        "ok" => s
            .services
            .iter()
            .any(|x| x.state == "error" || x.state == "connecting"),
        _ => false,
    }
}

/// The icon's tooltip: "Connectors", then what needs a look, or that
/// everything's signed in.
pub fn tooltip(s: Option<&ConnectorsStatus>) -> String {
    let mut lines = vec!["Connectors".to_owned()];
    match s.map(|s| s.state.as_str()) {
        Some("ok") => {
            let all = &s.expect("ok has a status").services;
            let out: Vec<&str> = signed_out(s).iter().map(|x| x.title.as_str()).collect();
            if !out.is_empty() {
                lines.push(format!(
                    "{}{}",
                    list(&out),
                    if out.len() == 1 {
                        " needs signing in"
                    } else {
                        " need signing in"
                    }
                ));
            }
            let broken: Vec<&str> = all
                .iter()
                .filter(|x| x.state == "error")
                .map(|x| x.title.as_str())
                .collect();
            if !broken.is_empty() {
                lines.push(format!(
                    "{}{}",
                    list(&broken),
                    if broken.len() == 1 {
                        " has a problem"
                    } else {
                        " have problems"
                    }
                ));
            }
            if out.is_empty() && broken.is_empty() {
                lines.push(
                    if all.is_empty() {
                        "No services yet"
                    } else {
                        "Everything is signed in"
                    }
                    .to_owned(),
                );
            }
        }
        Some("unreachable") => lines.push("Can't reach the gateway".to_owned()),
        Some("checking") => lines.push("Checking".to_owned()),
        _ => {}
    }
    lines.join("\n")
}

/// The panel's line when there are no services to list.
pub fn empty_line(s: Option<&ConnectorsStatus>) -> Option<String> {
    let Some(s) = s else {
        return Some("Checking the gateway.".to_owned());
    };
    match s.state.as_str() {
        "off" => Some("No Connectors gateway is set up for this runner.".to_owned()),
        "checking" => Some("Checking the gateway.".to_owned()),
        "unreachable" => {
            let host = host(Some(s)).unwrap_or_else(|| "the gateway".to_owned());
            Some(format!(
                "Can't reach {host}{}",
                s.error
                    .as_ref()
                    .map_or_else(|| ".".to_owned(), |e| format!(": {e}."))
            ))
        }
        _ => s
            .services
            .is_empty()
            .then(|| "The gateway has no services set up.".to_owned()),
    }
}

/// xSearch.status (docs/protocol.md, "X search"): whether X search is on
/// and an xAI key is saved. Never the key.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XSearchStatus {
    pub on: bool,
    pub key_saved: bool,
}

impl XSearchStatus {
    pub fn decode(v: &Value) -> XSearchStatus {
        XSearchStatus {
            on: v["on"].as_bool().unwrap_or(false),
            key_saved: v["keySaved"].as_bool().unwrap_or(false),
        }
    }

    /// The row's second line.
    pub fn line(&self) -> &'static str {
        if self.key_saved {
            "xAI key saved"
        } else {
            "Asks for an xAI key on the first search"
        }
    }
}

/// The gateway's machine, as its owner calls it: "bruce" for
/// bruce.example.ts.net.
pub fn host(s: Option<&ConnectorsStatus>) -> Option<String> {
    let (_, authority) = web_parts(s?.url.as_deref()?)?;
    let host = host_of(authority);
    if host.is_empty() {
        return None;
    }
    // A gateway on this Mac (what Set up makes) is "This Mac", not "127".
    if host == "localhost" || host == "::1" || host.starts_with("127.") {
        return Some("This Mac".to_owned());
    }
    Some(host.split('.').next().unwrap_or(host).to_owned())
}

/// The gateway's admin page, where every service can be connected, signed
/// out or retried, for the panel's manage button.
pub fn manage_url(s: Option<&ConnectorsStatus>) -> Option<String> {
    admin_url(s, "/admin")
}

/// One service's page on the gateway (its Connect, Disconnect or Try again),
/// whatever its state. The runner only sends a connect URL for a service that
/// isn't signed in, so a signed-in one gets the gateway's page for it, as the
/// runner's own fallback does.
pub fn service_url(s: Option<&ConnectorsStatus>, service: &ConnectorService) -> Option<String> {
    if let Some(raw) = service
        .connect_url
        .as_deref()
        .filter(|raw| web_parts(raw).is_some())
    {
        return Some(raw.to_owned());
    }
    admin_url(s, &format!("/admin/connect/{}", path_segment(&service.id)))
}

fn admin_url(s: Option<&ConnectorsStatus>, path: &str) -> Option<String> {
    let s = s.filter(|s| s.state != "off")?;
    let (scheme, authority) = web_parts(s.url.as_deref()?)?;
    Some(format!("{scheme}://{authority}{path}"))
}

/// A web address's scheme and the part before its path
/// ("bruce.example.ts.net:8443"), or None for anything but http(s), or
/// something that doesn't read as an address at all.
fn web_parts(raw: &str) -> Option<(&str, &str)> {
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let (scheme, rest) = raw.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    (!authority.is_empty()).then_some((scheme, authority))
}

/// The host in an authority, without a name and password or the port.
fn host_of(authority: &str) -> &str {
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    match host.strip_prefix('[') {
        Some(inner) => inner.split(']').next().unwrap_or(inner),
        None => host.split(':').next().unwrap_or(host),
    }
}

/// An id as one piece of a path: everything but the characters a path keeps
/// as they are (`urlPathAllowed`, less "/") is percent-encoded.
fn path_segment(id: &str) -> String {
    let mut out = String::new();
    for b in id.bytes() {
        if b.is_ascii_alphanumeric() || b"!$&'()*+,-.:;=@_~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A service's state in a word or two.
pub fn state_line(service: &ConnectorService) -> String {
    if service.local && service.state == "connected" {
        return "Running on this Mac".into();
    }
    match service.state.as_str() {
        "connected" => "Signed in".to_owned(),
        "signed_out" => "Signed out".to_owned(),
        "connecting" => "Connecting".to_owned(),
        "error" => "Not working".to_owned(),
        "disabled" => "Turned off".to_owned(),
        other => {
            let words = other.replace('_', " ");
            let mut chars = words.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        }
    }
}

/// The button beside a service, or None when there's nothing to press.
pub fn button_title(service: &ConnectorService) -> Option<&'static str> {
    // Connect would only fail without its app: Create comes first.
    if service.local || service.client_setup.is_some() {
        return None;
    }
    service.connect_url.as_deref().filter(|u| !u.is_empty())?;
    match service.state.as_str() {
        "connected" | "disabled" => None,
        "signed_out" => Some("Connect"),
        _ => Some("Reconnect"),
    }
}

/// The ⓘ's words: where the services come from, and the claude.ai ones this
/// panel doesn't cover.
pub fn about(s: Option<&ConnectorsStatus>) -> [String; 3] {
    let on = host(s).map_or_else(String::new, |h| format!(" on {h}"));
    [
        format!("These are the services your agents get from the Connectors gateway{on}. Connect opens a service's sign-in page in your browser. The arrows open the gateway's own pages, to sign a service out or reconnect it."),
        "The runner checks every few minutes and when this panel opens. When an agent runs into a service that needs signing in, it asks you with a link to the page.".to_owned(),
        "claude.ai's own connectors are signed in on claude.ai, for each login.".to_owned(),
    ]
}

/// "Gmail", "Gmail and Notion", "Gmail, Notion and Sentry".
fn list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// What the mock runner's gateway says (app/Tools/mock-runner/server.mjs).
    fn mock() -> ConnectorsStatus {
        ConnectorsStatus::decode(
            &json!({ "state": "ok", "url": "https://mac-mini.example.ts.net:8443", "version": "0.2.0", "checkedAt": "2026-11-11T01:00:00.000Z", "services": [
                { "id": "notion", "title": "Notion", "state": "connected" },
                { "id": "sentry", "title": "Sentry", "state": "signed_out", "message": "The sign-in expired.", "connectUrl": "https://mac-mini.example.ts.net:8443/admin/connect/sentry" },
                { "id": "cloudflare", "state": "connected" },
                { "title": "No id" },
            ]}),
        )
    }

    #[test]
    fn a_signed_out_service_wants_mike() {
        let s = mock();
        assert_eq!(s.services.len(), 3);
        assert_eq!(s.services[2].title, "cloudflare", "no title is its id");
        assert!(needs_you(Some(&s)));
        assert!(!quiet(Some(&s)));
        assert_eq!(tooltip(Some(&s)), "Connectors\nSentry needs signing in");
        assert_eq!(empty_line(Some(&s)), None);
        assert_eq!(state_line(&s.services[1]), "Signed out");
        assert_eq!(button_title(&s.services[1]), Some("Connect"));
        assert_eq!(button_title(&s.services[0]), None);
        assert_eq!(host(Some(&s)).as_deref(), Some("mac-mini"));
        assert_eq!(
            manage_url(Some(&s)).as_deref(),
            Some("https://mac-mini.example.ts.net:8443/admin")
        );
        assert_eq!(
            service_url(Some(&s), &s.services[1]).as_deref(),
            Some("https://mac-mini.example.ts.net:8443/admin/connect/sentry")
        );
        assert_eq!(
            service_url(Some(&s), &s.services[0]).as_deref(),
            Some("https://mac-mini.example.ts.net:8443/admin/connect/notion"),
            "a signed-in one gets its page on the gateway"
        );
        assert!(about(Some(&s))[0].contains("Connectors gateway on mac-mini."));
    }

    #[test]
    fn trouble_is_the_grey_dot() {
        let mut s = mock();
        s.services[1].state = "error".into();
        assert!(!needs_you(Some(&s)));
        assert!(quiet(Some(&s)));
        assert_eq!(button_title(&s.services[1]), Some("Reconnect"));
        assert_eq!(tooltip(Some(&s)), "Connectors\nSentry has a problem");
        let down = ConnectorsStatus::decode(
            &json!({ "state": "unreachable", "url": "https://mac-mini.example.ts.net:8443", "error": "timed out", "services": [] }),
        );
        assert!(quiet(Some(&down)));
        assert_eq!(tooltip(Some(&down)), "Connectors\nCan't reach the gateway");
        assert_eq!(
            empty_line(Some(&down)).as_deref(),
            Some("Can't reach mac-mini: timed out.")
        );
        let fine = ConnectorsStatus {
            services: vec![ConnectorService {
                id: "n".into(),
                title: "Notion".into(),
                state: "connected".into(),
                ..Default::default()
            }],
            ..mock()
        };
        assert_eq!(tooltip(Some(&fine)), "Connectors\nEverything is signed in");
        assert!(!needs_you(Some(&fine)) && !quiet(Some(&fine)));
        let off = ConnectorsStatus::decode(&json!({ "state": "off" }));
        assert_eq!(
            empty_line(Some(&off)).as_deref(),
            Some("No Connectors gateway is set up for this runner.")
        );
        assert_eq!(manage_url(Some(&off)), None);
        assert_eq!(empty_line(None).as_deref(), Some("Checking the gateway."));
        assert_eq!(
            state_line(&ConnectorService {
                state: "needs_more_love".into(),
                ..Default::default()
            }),
            "Needs more love"
        );
    }

    #[test]
    fn x_search_says_whether_a_key_is_saved() {
        let saved = XSearchStatus::decode(&json!({ "on": true, "keySaved": true }));
        assert_eq!(
            saved,
            XSearchStatus {
                on: true,
                key_saved: true
            }
        );
        assert_eq!(saved.line(), "xAI key saved");
        let odd = XSearchStatus::decode(&json!({ "on": "yes" }));
        assert!(!odd.on && !odd.key_saved);
        assert_eq!(odd.line(), "Asks for an xAI key on the first search");
    }

    #[test]
    fn only_web_addresses_are_opened() {
        let mut s = mock();
        s.url = Some("file:///etc/passwd".into());
        assert_eq!(manage_url(Some(&s)), None);
        assert_eq!(host(Some(&s)), None);
        s.url = Some("http://localhost:7000/some/path?x#y".into());
        assert_eq!(
            manage_url(Some(&s)).as_deref(),
            Some("http://localhost:7000/admin")
        );
        let odd = ConnectorService {
            id: "a/b c".into(),
            connect_url: Some("javascript:alert(1)".into()),
            ..Default::default()
        };
        assert_eq!(
            service_url(Some(&s), &odd).as_deref(),
            Some("http://localhost:7000/admin/connect/a%2Fb%20c")
        );
        s.url = Some("https://[::1]:8443".into());
        assert_eq!(host(Some(&s)).as_deref(), Some("This Mac"));
    }

    #[test]
    fn a_local_gateway_is_this_mac() {
        let mut s = mock();
        for url in ["http://127.0.0.1:8795", "http://localhost:8795"] {
            s.url = Some(url.into());
            assert_eq!(host(Some(&s)).as_deref(), Some("This Mac"), "{url}");
        }
        s.url = Some("https://mac-mini.tail.ts.net:8443".into());
        assert_eq!(host(Some(&s)).as_deref(), Some("mac-mini"));
    }
}

/// The Create button's words: "Create Slack app".
pub fn create_app_title(title: &str) -> String {
    format!("Create {title} app")
}

/// What the paste form says about what's in it: whether Save can go, and a
/// warning when it doesn't look like what the service usually gives. The
/// runner refuses only blank, spaced or absurd keys, so an odd one still
/// saves, in case the service has changed its formats.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PasteCheck {
    pub can_save: bool,
    pub warning: Option<String>,
}

pub fn paste_check(id: &str, client_id: &str, secret: &str) -> PasteCheck {
    let (client_id, secret) = (client_id.trim(), secret.trim());
    let spaced = |s: &str| s.chars().any(char::is_whitespace);
    if client_id.is_empty() || secret.is_empty() {
        return PasteCheck {
            can_save: false,
            warning: None,
        };
    }
    if spaced(client_id) || spaced(secret) {
        return PasteCheck {
            can_save: false,
            warning: Some("There's a space in it: copy it again.".into()),
        };
    }
    if id == "slack" {
        let id_ok = client_id.split_once('.').is_some_and(|(a, b)| {
            !a.is_empty()
                && !b.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.bytes().all(|c| c.is_ascii_digit())
        });
        let secret_ok = secret.len() == 32 && secret.bytes().all(|c| c.is_ascii_hexdigit());
        let warning = match (id_ok, secret_ok) {
            (true, true) => None,
            (false, _) => Some("That doesn't look like a Slack Client ID (numbers with a dot in the middle). Check you copied the Client ID.".into()),
            (_, false) => Some("That doesn't look like a Slack Client Secret (32 letters and numbers). Check you copied the Secret, not the Signing Secret.".into()),
        };
        return PasteCheck {
            can_save: true,
            warning,
        };
    }
    PasteCheck {
        can_save: true,
        warning: None,
    }
}
