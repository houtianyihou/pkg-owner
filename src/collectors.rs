use crate::{process::run, Record};
use serde_json::Value;
use std::os::unix::ffi::OsStrExt;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn home() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}
pub fn expand(s: &str) -> PathBuf {
    if s == "~" {
        home()
    } else if let Some(rest) = s.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(s)
    }
}
pub fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.into()
    } else {
        env::current_dir().unwrap_or_default().join(p)
    }
}
pub fn resolved(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap_or_else(|_| absolute(p))
}
pub fn display(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
pub fn entries(p: &Path) -> Vec<PathBuf> {
    fs::read_dir(p)
        .map(|it| it.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default()
}
pub fn executable(p: &Path) -> bool {
    if !p.is_file() {
        return false;
    }
    let Ok(path) = std::ffi::CString::new(p.as_os_str().as_bytes()) else {
        return false;
    };
    // Match actual access, including ACLs, rather than only Unix mode bits.
    unsafe { libc::access(path.as_ptr(), libc::X_OK) == 0 }
}

pub fn which(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|p| p.join(name))
        .find(|p| executable(p))
}
fn json(args: &[&str]) -> Result<Value, String> {
    serde_json::from_str(&run(args)?).map_err(|e| e.to_string())
}
fn array(v: &Value) -> Vec<&Value> {
    v.as_array().map(|a| a.iter().collect()).unwrap_or_default()
}
fn strings(v: &Value) -> Vec<String> {
    array(v)
        .iter()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect()
}
fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn brew() -> Result<Vec<Record>, String> {
    let data = json(&["brew", "info", "--json=v2", "--installed"])?;
    let binary = resolved(&which("brew").ok_or("brew not found")?);
    let prefix = binary
        .parent()
        .and_then(Path::parent)
        .ok_or("invalid brew path")?;
    Ok(parse_brew(&data, prefix))
}
fn parse_brew(data: &Value, prefix: &Path) -> Vec<Record> {
    let mut rows = vec![];
    for f in array(&data["formulae"]) {
        let root = prefix.join("Cellar").join(text(&f["name"]));
        let mut paths = vec![display(&root)];
        for installed in entries(&root) {
            for folder in ["bin", "sbin"] {
                paths.extend(entries(&installed.join(folder)).iter().map(|p| display(p)));
            }
        }
        let mut row = Record::new(
            text(&f["name"]),
            "Homebrew Formula",
            "brew installed record",
            paths,
        );
        row.aliases = strings(&f["aliases"]);
        row.aliases.extend(row.paths.iter().skip(1).filter_map(|p| {
            Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        }));
        rows.push(row);
    }
    for c in array(&data["casks"]) {
        let mut paths = vec![];
        for artifact in array(&c["artifacts"]) {
            for kind in ["app", "binary"] {
                let values = array(&artifact[kind]);
                if let Some(source) = values.first().and_then(|v| v.as_str()) {
                    if let Some(target) = values.iter().skip(1).find_map(|v| v["target"].as_str()) {
                        paths.push(display(&expand(target)));
                    } else if kind == "app" {
                        for base in [PathBuf::from("/Applications"), home().join("Applications")] {
                            let p = base.join(source);
                            if p.exists() {
                                paths.push(display(&p));
                            }
                        }
                    }
                }
            }
        }
        let mut row = Record::new(
            text(&c["token"]),
            "Homebrew Cask",
            "brew installed record",
            paths,
        );
        row.aliases = strings(&c["name"]);
        rows.push(row);
    }
    rows
}
pub const MANAGERS: [&str; 6] = ["brew", "npm", "pipx", "uv", "cargo", "port"];
pub fn collect(name: &str) -> Result<Vec<Record>, String> {
    match name {
        "brew" => brew(),
        "npm" => {
            let data = json(&["npm", "ls", "-g", "--depth=0", "--json"])?;
            let root = PathBuf::from(run(&["npm", "root", "-g"])?.trim());
            Ok(data["dependencies"]
                .as_object()
                .into_iter()
                .flat_map(|o| o.keys())
                .map(|n| Record::new(n, "npm global", "npm ls -g", vec![display(&root.join(n))]))
                .collect())
        }
        "pipx" => {
            let data = json(&["pipx", "list", "--json"])?;
            Ok(data["venvs"]
                .as_object()
                .into_iter()
                .flat_map(|o| o.iter())
                .map(|(n, v)| {
                    let mut row = Record::new(n, "pipx", "pipx list", vec![]);
                    row.aliases = strings(&v["metadata"]["main_package"]["apps"]);
                    row
                })
                .collect())
        }
        "uv" => {
            let base = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home().join(".local/share"));
            let root = env::var("UV_TOOL_DIR")
                .map(|s| expand(&s))
                .unwrap_or_else(|_| base.join("uv/tools"));
            Ok(entries(&root)
                .iter()
                .filter(|p| p.join("uv-receipt.toml").is_file())
                .map(|p| {
                    Record::new(
                        &p.file_name().unwrap_or_default().to_string_lossy(),
                        "uv tool",
                        "uv-receipt.toml 安装记录",
                        vec![display(p)],
                    )
                })
                .collect())
        }
        "cargo" | "port" => {
            let (args, manager, evidence): (&[&str], _, _) = if name == "cargo" {
                (
                    &["cargo", "install", "--list"],
                    "Cargo",
                    "cargo install --list",
                )
            } else {
                (
                    &["port", "installed"],
                    "MacPorts",
                    "port installed (active)",
                )
            };
            Ok(run(args)?
                .lines()
                .filter(|l| {
                    if name == "cargo" {
                        !l.is_empty() && !l.starts_with(char::is_whitespace)
                    } else {
                        l.contains("(active)")
                    }
                })
                .filter_map(|l| l.split_whitespace().next())
                .map(|n| Record::new(n, manager, evidence, vec![]))
                .collect())
        }
        _ => Err(format!("unsupported manager: {name}")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn brew_records_and_targets() {
        let data = serde_json::json!({"formulae":[{"name":"example", "aliases":["alias"]}], "casks":[{"token":"app", "name":["My App"], "artifacts":[{"app":["App.app",{"target":"/custom/My App.app"}]}]}]});
        let rows = parse_brew(&data, Path::new("/nonexistent-prefix"));
        assert_eq!(rows[0].paths, ["/nonexistent-prefix/Cellar/example"]);
        assert_eq!(rows[0].aliases, ["alias"]);
        assert_eq!(rows[1].paths, ["/custom/My App.app"]);
    }
}
