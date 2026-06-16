//! Verify the agent-info manifest matches reality.
//!
//! Every command listed in agent-info must be routable, the four core Elicit
//! commands (search / trials / report / review) must be present with argument
//! schemas, and the manifest must carry exit codes, the envelope, and the
//! config block (with ELICIT_API_KEY). Drift here is a P0 bug.

use assert_cmd::Command;

fn elicit() -> Command {
    Command::cargo_bin("elicit").unwrap()
}

fn agent_info() -> serde_json::Value {
    let out = elicit().arg("agent-info").output().unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).expect("agent-info must be valid JSON")
}

/// Find a command entry by a prefix of its key (keys include the positional
/// signature, e.g. "search <query>").
fn command_with_prefix<'a>(
    info: &'a serde_json::Value,
    prefix: &str,
) -> Option<&'a serde_json::Value> {
    info["commands"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(k, _)| k.starts_with(prefix))
        .map(|(_, v)| v)
}

// ── Required top-level fields ──────────────────────────────────────────────

#[test]
fn has_required_fields() {
    let info = agent_info();
    assert!(info["name"].is_string());
    assert!(info["version"].is_string());
    assert!(info["description"].is_string());
    assert!(info["commands"].is_object());
    assert!(info["exit_codes"].is_object());
    assert!(info["envelope"].is_object());
    assert!(info["auto_json_when_piped"].is_boolean());
}

#[test]
fn name_matches_binary() {
    let info = agent_info();
    assert_eq!(info["name"], "elicit");
}

#[test]
fn at_least_eight_commands() {
    // The proofs require >= 8 commands; the eight endpoints plus built-ins.
    let info = agent_info();
    let n = info["commands"].as_object().unwrap().len();
    assert!(n >= 8, "expected >= 8 commands, found {n}");
}

// ── Exit codes ─────────────────────────────────────────────────────────────

#[test]
fn exit_codes_cover_full_contract() {
    let info = agent_info();
    let codes = &info["exit_codes"];
    for code in ["0", "1", "2", "3", "4"] {
        assert!(
            codes[code].is_string(),
            "exit_codes must document code {code}"
        );
    }
}

// ── Envelope + config block ────────────────────────────────────────────────

#[test]
fn envelope_documented() {
    let info = agent_info();
    let env = &info["envelope"];
    assert_eq!(env["version"], "1");
    assert!(env["success"].is_string());
    assert!(env["error"].is_string());
}

#[test]
fn config_block_names_elicit_api_key() {
    let info = agent_info();
    let config = &info["config"];
    assert!(config["path"].is_string());
    assert_eq!(config["env_prefix"], "ELICIT_");
    assert_eq!(config["api_key_env"], "ELICIT_API_KEY");
    // Resolution order must be documented for agents.
    assert!(config["api_key_resolution"].is_array());
}

#[test]
fn global_flags_documented() {
    let info = agent_info();
    let flags = &info["global_flags"];
    assert!(flags["--json"].is_object());
    assert!(flags["--quiet"].is_object());
    assert!(flags["--api-key"].is_object());
}

// ── The four core Elicit commands: present + arg schema ────────────────────

#[test]
fn search_present_with_arg_schema() {
    let info = agent_info();
    let cmd = command_with_prefix(&info, "search").expect("search must be listed");
    let args = cmd["args"].as_array().expect("search must have args array");
    assert!(!args.is_empty(), "search must document its <query> arg");
    assert_eq!(args[0]["name"], "query");
    assert_eq!(args[0]["required"], true);
    // Options must include the documented filter flags.
    let opts = cmd["options"].as_array().unwrap();
    let opt_names: Vec<&str> = opts.iter().filter_map(|o| o["name"].as_str()).collect();
    assert!(opt_names.contains(&"--corpus"));
    assert!(opt_names.contains(&"--max-results"));
    assert!(opt_names.contains(&"--type"));
}

#[test]
fn trials_present_with_arg_schema() {
    let info = agent_info();
    let cmd = command_with_prefix(&info, "trials").expect("trials must be listed");
    let args = cmd["args"].as_array().expect("trials must have args array");
    assert!(!args.is_empty());
    assert_eq!(args[0]["name"], "query");
    assert_eq!(args[0]["required"], true);
    let opts = cmd["options"].as_array().unwrap();
    let opt_names: Vec<&str> = opts.iter().filter_map(|o| o["name"].as_str()).collect();
    assert!(opt_names.contains(&"--phase"));
    assert!(opt_names.contains(&"--status"));
}

#[test]
fn report_subcommands_present_with_arg_schema() {
    let info = agent_info();
    let new = command_with_prefix(&info, "report new").expect("report new must be listed");
    let args = new["args"].as_array().expect("report new must have args");
    assert_eq!(args[0]["name"], "question");
    assert_eq!(args[0]["required"], true);

    let get = command_with_prefix(&info, "report get").expect("report get must be listed");
    assert!(get["args"].is_array());
    assert!(command_with_prefix(&info, "report list").is_some());
}

#[test]
fn review_subcommands_present_with_arg_schema() {
    let info = agent_info();
    let new = command_with_prefix(&info, "review new").expect("review new must be listed");
    let args = new["args"].as_array().expect("review new must have args");
    assert_eq!(args[0]["name"], "question");
    assert_eq!(args[0]["required"], true);

    let get = command_with_prefix(&info, "review get").expect("review get must be listed");
    let opts = get["options"].as_array().unwrap();
    let opt_names: Vec<&str> = opts.iter().filter_map(|o| o["name"].as_str()).collect();
    assert!(opt_names.contains(&"--download"));
    assert!(opt_names.contains(&"--stage"));
    assert!(command_with_prefix(&info, "review list").is_some());
}

// ── Every listed command is routable ───────────────────────────────────────
//
// "Routable" = the binary recognizes the command path and does NOT fail with a
// clap parse/usage error (exit 3 from an unknown subcommand). API commands
// without a key fail-fast with exit 2, which still proves the route exists.
// We never let any of these touch the network: API routes are checked only on
// the no-key (exit 2) path with an unroutable base_url as a guard.

fn assert_routable(args: &[&str], allowed_codes: &[i32]) {
    let tmp = tempfile::tempdir().unwrap();
    let out = elicit()
        .env_remove("ELICIT_API_KEY")
        .env("HOME", tmp.path())
        .env("ELICIT_BASE_URL", "http://127.0.0.1:1/never")
        .args(args)
        .output()
        .unwrap();
    let code = out.status.code().unwrap_or(-1);
    assert!(
        allowed_codes.contains(&code),
        "`elicit {}` exited {code}, expected one of {allowed_codes:?}",
        args.join(" ")
    );
}

#[test]
fn builtins_are_routable() {
    assert_routable(&["agent-info"], &[0]);
    assert_routable(&["info"], &[0]);
    assert_routable(&["config", "show"], &[0]);
    assert_routable(&["config", "path"], &[0]);
    assert_routable(&["skill", "status"], &[0]);
    assert_routable(&["skill", "install"], &[0]);
    assert_routable(&["doctor"], &[2]); // no key -> exit 2, offline-safe
}

#[test]
fn core_commands_are_routable_without_network() {
    // No key resolvable -> exit 2 fail-fast (route exists, gate fires first).
    assert_routable(&["search", "q"], &[2]);
    assert_routable(&["trials", "q"], &[2]);
    assert_routable(&["report", "new", "q"], &[2]);
    assert_routable(&["report", "list"], &[2]);
    assert_routable(&["report", "get", "some-id"], &[2]);
    assert_routable(&["review", "new", "q"], &[2]);
    assert_routable(&["review", "list"], &[2]);
    assert_routable(&["review", "get", "some-id"], &[2]);
}

#[test]
fn crud_aliases_are_routable() {
    // visible_alias ls / show must resolve to the same routes.
    assert_routable(&["report", "ls"], &[2]);
    assert_routable(&["report", "show", "some-id"], &[2]);
    assert_routable(&["review", "ls"], &[2]);
    assert_routable(&["review", "show", "some-id"], &[2]);
}
