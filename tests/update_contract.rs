//! Verify the distribution-aware update contract.
//!
//! The tests force managed/package-manager channels through config so they do
//! not hit the network or mutate the machine running the tests.

use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn elicit() -> Command {
    Command::cargo_bin("elicit").unwrap()
}

fn config_path_for_home(home: &Path) -> PathBuf {
    let out = elicit()
        .env("HOME", home)
        .args(["--json", "config", "path"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("config path should be JSON");
    PathBuf::from(json["data"]["path"].as_str().unwrap())
}

fn write_config(home: &Path, contents: &str) {
    let path = config_path_for_home(home);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn update_check_with_config(config: &str) -> serde_json::Value {
    let tmp = tempfile::tempdir().unwrap();
    write_config(tmp.path(), config);

    let out = elicit()
        .env("HOME", tmp.path())
        .args(["--json", "update", "--check"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(0));
    serde_json::from_slice(&out.stdout).expect("update --check should emit JSON")
}

#[test]
fn disabled_update_returns_disabled_status() {
    let json = update_check_with_config(
        r#"
[update]
enabled = false
install_source = "managed"
"#,
    );

    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["status"], "disabled");
    assert_eq!(json["data"]["install_source"], "managed");
    assert_eq!(json["data"]["update_mode"], "disabled");
}

#[test]
fn homebrew_update_returns_brew_upgrade_command() {
    let json = update_check_with_config(
        r#"
[update]
install_source = "homebrew"
formula = "elicit"
tap = "paperfoot/tap"
"#,
    );

    assert_eq!(json["data"]["status"], "managed_install");
    assert_eq!(json["data"]["install_source"], "homebrew");
    assert_eq!(json["data"]["update_mode"], "package_manager");
    assert_eq!(
        json["data"]["upgrade_command"],
        "brew upgrade paperfoot/tap/elicit"
    );
}

#[test]
fn cargo_update_returns_cargo_install_command() {
    let json = update_check_with_config(
        r#"
[update]
install_source = "cargo"
crate_name = "elicit"
"#,
    );

    assert_eq!(json["data"]["status"], "managed_install");
    assert_eq!(json["data"]["install_source"], "cargo");
    assert_eq!(
        json["data"]["upgrade_command"],
        "cargo install --locked --force elicit"
    );
}

#[test]
fn uv_tool_update_returns_uv_upgrade_command() {
    let json = update_check_with_config(
        r#"
[update]
install_source = "uv_tool"
crate_name = "elicit"
"#,
    );

    assert_eq!(json["data"]["status"], "managed_install");
    assert_eq!(json["data"]["install_source"], "uv_tool");
    assert_eq!(json["data"]["upgrade_command"], "uv tool upgrade elicit");
}

#[test]
fn bun_update_returns_bun_global_update_command() {
    let json = update_check_with_config(
        r#"
[update]
install_source = "bun"
crate_name = "elicit"
"#,
    );

    assert_eq!(json["data"]["status"], "managed_install");
    assert_eq!(json["data"]["install_source"], "bun");
    assert_eq!(
        json["data"]["upgrade_command"],
        "bun update --global elicit"
    );
}

#[test]
fn invalid_update_source_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    write_config(
        tmp.path(),
        r#"
[update]
install_source = "spaceship"
"#,
    );

    let out = elicit()
        .env("HOME", tmp.path())
        .args(["--json", "update", "--check"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("config error should be JSON");
    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "config_error");
}

#[test]
fn agent_info_documents_update_contract_shape() {
    let out = elicit().arg("agent-info").output().unwrap();
    assert!(out.status.success());
    let info: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    let update = &info["commands"]["update"];
    assert!(
        update["description"]
            .as_str()
            .unwrap()
            .starts_with("Distribution-aware update check/apply."),
        "update description should describe distribution-aware check/apply, got: {}",
        update["description"]
    );
    assert!(
        update["install_sources"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("homebrew".into()))
    );
    assert!(
        update["install_sources"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("uv_tool".into()))
    );
    assert!(
        update["install_sources"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("bun".into()))
    );
    assert!(
        update["data_fields"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("upgrade_command".into()))
    );
}
