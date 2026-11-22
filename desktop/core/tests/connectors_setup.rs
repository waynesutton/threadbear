use serde_json::json;
use threadbear_core::connectors::*;

#[test]
fn setup_agents_and_catalog_decode_without_breaking_old_runners() {
    let status = ConnectorsStatus::decode(
        &json!({"state":"off", "canSetUp":true, "canAddLocal":true, "catalog":[{"id":"blender","kind":"local","command":"uvx mcp-for-blender@2.2.7","asks":["blender__execute_blender_code"]}, {"title":"missing identity"}]}),
    );
    assert!(shows_icon(Some(&status), None));
    assert!(status.can_add_local);
    assert_eq!(status.catalog.len(), 1);
    assert!(status.catalog[0].is_local());
    assert!(!shows_icon(
        Some(&ConnectorsStatus::decode(&json!({"state":"off"}))),
        None
    ));
    assert!(shows_icon(
        None,
        Some(&AgentsList::decode(&json!({"on":true})))
    ));
    let agents = AgentsList::decode(
        &json!({"on":true,"reason":"Sync is off","keys":[{"id":"k","name":"Notion","scope":"write","readAll":true},{"name":"no id"}]}),
    );
    assert_eq!(agents.reason.as_deref(), Some("Sync is off"));
    assert_eq!(agents.keys.len(), 1);
    assert_eq!(
        scope_line(&agents.keys[0]),
        "Starts Ask threads, reads all your threads"
    );
    assert_eq!(last_used(&agents.keys[0], 0), "Not used yet");
}

#[test]
fn keys_are_masked_but_copy_the_complete_header() {
    let secret = "tba_private";
    let fields = OutsideAgentApp::Grok.form("https://demo.convex.site/mcp", secret);
    assert_eq!(
        fields.iter().map(|f| f.label).collect::<Vec<_>>(),
        [
            "Name",
            "Type",
            "Server URL",
            "Header key",
            "Header value",
            "Client ID"
        ]
    );
    assert!(fields.iter().all(|f| !f.shown.contains("private")));
    assert_eq!(fields[4].shown, "Bearer tba_••••••••");
    assert_eq!(fields[4].value, "Bearer tba_private");
    assert!(!fields[1].copies && !fields[5].copies);
    let notion = OutsideAgentApp::Notion.form("https://demo.convex.site/mcp", secret);
    assert_eq!(notion[2].label, "Auth");
    assert!(!OutsideAgentApp::Slack.can_connect());
    assert!(OutsideAgentApp::Slack.form("url", secret).is_empty());
}

#[test]
fn last_used_uses_the_same_thresholds_as_swift() {
    let mut key = AgentKey::decode(&json!({"id":"k","lastUsedAt":"2026-11-14T00:00:00Z"})).unwrap();
    let at = threadbear_core::inbox::parse_ms("2026-11-14T00:00:00Z").unwrap();
    for (mins, expected) in [
        (0, "Used just now"),
        (2, "Used 2 minutes ago"),
        (60, "Used 1 hours ago"),
        (2880, "Used 2 days ago"),
    ] {
        assert_eq!(last_used(&key, at + mins * 60_000), expected);
    }
    key.expired = true;
    assert_eq!(last_used(&key, at), "Expired");
}

#[test]
fn hiding_only_changes_the_panel_and_indicators() {
    let status = ConnectorsStatus::decode(
        &json!({"state":"ok", "services":[{"id":"sentry","state":"signed_out"},{"id":"broken","state":"error"}], "catalog":[{"id":"slack","title":"Slack"}]}),
    );
    let hidden = hidden_decode(
        " service:sentry\nservice:broken\ncatalog:slack\nagent:grok-bot\nservice:sentry\n",
    );
    assert_eq!(hidden.len(), 4);
    assert_eq!(hidden_decode(&hidden_encode(&hidden)), hidden);
    let visible = visible_status(Some(&status), &hidden).unwrap();
    assert!(!needs_you(Some(&visible)) && !quiet(Some(&visible)));
    assert!(!tooltip(Some(&visible)).contains("sentry"));
    assert_eq!(status.services.len(), 2);
    let agents = AgentsList::decode(&json!({"on":true,"keys":[{"id":"k"}]}));
    assert_eq!(hidden_items(&hidden, Some(&status), Some(&agents)).len(), 4);
    assert!(
        show_agents(Some(&agents), &hidden),
        "a key's Revoke must stay reachable"
    );
    assert!(!show_agents(
        Some(&AgentsList {
            keys: vec![],
            ..agents
        }),
        &hidden
    ));
}

#[test]
fn local_servers_need_exact_pins_and_explain_trust() {
    for (launcher, package, version, good) in [
        ("uvx", "mcp-for-blender", "2.2.7", true),
        ("npx", "@scope/server", "1.2.3-beta.1", true),
        ("uvx", "server", "1.2rc1.post2.dev3", true),
        ("npx", "server", "1.2", false),
        ("uvx", "server", "latest", false),
        ("npx", "server", "^1.2.3", false),
        ("sh", "server", "1.2.3", false),
        ("uvx", "https://host/pkg", "1.2.3", false),
        ("uvx", "pkg;touch /tmp/x", "1.2.3", false),
        ("npx", "server", "1.2.3+build", false),
    ] {
        assert_eq!(
            LocalServer::new(launcher, package, version, "").is_pinned(),
            good,
            "{launcher} {package} {version}"
        );
    }
    assert_eq!(
        LocalServer::new("npx", " @scope/server ", " 1.2.3 ", "").command(),
        "npx -y @scope/server@1.2.3"
    );
    assert!(trust_lines(&["blender__execute_blender_code".into()])[1]
        .contains("blender__execute_blender_code"));
    assert!(trust_lines(&[])[1].starts_with("None of its tools ask first"));
    let status = ConnectorsStatus::decode(
        &json!({"services":[{"id":"local","local":true,"state":"connected","connectUrl":"https://host/"}]}),
    );
    assert_eq!(state_line(&status.services[0]), "Running on this Mac");
    assert_eq!(button_title(&status.services[0]), None);
}
