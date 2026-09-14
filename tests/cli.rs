use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
static COUNTER: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pkg-owner-cli-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("bin")).unwrap();
        Self(path)
    }
    fn script(&self, name: &str, body: &str) {
        let path = self.0.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pkg-owner"))
            .args(args)
            .env("PATH", self.0.join("bin"))
            .env("HOME", &self.0)
            .env("UV_TOOL_DIR", self.0.join("uv"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn real_cli_aliases_collectors_and_symlinks() {
    let f = Fixture::new();
    f.script("brew", r#"printf '%s' '{"formulae":[{"name":"fixture-package","aliases":["fixture-alias"]}],"casks":[]}'"#);
    let binary = f.0.join("Cellar/fixture-package/1/bin/fixture-command");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::write(&binary, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&binary, f.0.join("bin/fixture-command")).unwrap();
    f.script("pipx",r#"printf '%s' '{"venvs":{"fixture-pipx":{"metadata":{"main_package":{"apps":["fixture-pipx-cli"]}}}}}'"#);
    f.script(
        "cargo",
        "printf 'fixture-cargo v1.0.0:\n    fixture-cargo-cli\n'",
    );
    f.script(
        "port",
        "printf '  fixture-port @1.0 (active)\n  old @0.1 (inactive)\n'",
    );
    f.script("uv", "exit 99"); // uv must never be invoked: receipt-only collector.
    fs::create_dir_all(f.0.join("uv/fixture-uv")).unwrap();
    fs::write(f.0.join("uv/fixture-uv/uv-receipt.toml"), "").unwrap();
    f.script("npm",r#"if [ "$1" = root ]; then printf '%s' "$HOME/modules"; else printf '%s' '{"dependencies":{"fixture-npm":{}}}'; fi"#);
    f.script("mdfind", "exit 0");
    let short = f.run(&["-s", "fixture", "-j"]);
    let long = f.run(&["--search", "fixture", "--json"]);
    assert!(short.status.success());
    let a: Value = serde_json::from_slice(&short.stdout).unwrap();
    let b: Value = serde_json::from_slice(&long.stdout).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["warnings"], serde_json::json!([]));
    for manager in [
        "Homebrew Formula",
        "npm global",
        "pipx",
        "uv tool",
        "Cargo",
        "MacPorts",
    ] {
        assert!(
            a["results"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["manager"] == manager),
            "missing {manager}"
        );
    }
    let exact: Value = serde_json::from_slice(&f.run(&["fixture-command", "-j"]).stdout).unwrap();
    assert_eq!(exact["results"][0]["manager"], "Homebrew Formula");
    assert_eq!(f.run(&["-l", "-s", "fixture"]).status.code(), Some(2));
    assert_eq!(f.run(&["--no-such-option"]).status.code(), Some(2));
    assert!(f.run(&["-h"]).status.success());
}
#[test]
fn failures_custom_apps_and_not_found() {
    let f = Fixture::new();
    f.script("brew", "echo broken >&2; exit 1");
    f.script("mdfind", "exit 1");
    let app = f.0.join("elsewhere/FixtureUniqueApp.app");
    fs::create_dir_all(app.join("Contents/_MASReceipt")).unwrap();
    fs::write(app.join("Contents/_MASReceipt/receipt"), "").unwrap();
    let out = f.run(&[
        "-s",
        "FixtureUniqueApp",
        "-d",
        app.parent().unwrap().to_str().unwrap(),
        "-j",
    ]);
    assert!(out.status.success());
    let data: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(data["results"][0]["manager"], "App Store");
    assert!(data["warnings"][0].as_str().unwrap().contains("broken"));
    let missing = f.run(&["fixture-definitely-does-not-exist-785319", "-j"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(Path::new(&app).exists());
}
