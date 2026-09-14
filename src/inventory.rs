use crate::{collectors::*, process::run, Record};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    thread,
};

#[derive(Serialize)]
pub struct Inventory {
    pub results: Vec<Record>,
    pub warnings: Vec<String>,
    pub managers_checked: Vec<String>,
}
fn is_app(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "app")
}
pub fn scan_root(root: &Path, apps: &mut BTreeSet<PathBuf>) {
    if !root.is_dir() {
        return;
    }
    if is_app(root) {
        apps.insert(root.into());
        return;
    }
    for p in entries(root) {
        if is_app(&p) && p.is_dir() {
            apps.insert(p);
        } else if fs::symlink_metadata(&p).is_ok_and(|m| m.is_dir()) {
            scan_root(&p, apps);
        }
    }
}
fn scan_apps(extra: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut apps = BTreeSet::new();
    if let Ok(output) = run(&[
        "mdfind",
        "kMDItemContentType == 'com.apple.application-bundle'",
    ]) {
        apps.extend(
            output
                .lines()
                .map(PathBuf::from)
                .filter(|p| is_app(p) && p.is_dir()),
        );
    }
    for root in [
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        home().join("Applications"),
    ]
    .iter()
    .chain(extra)
    {
        scan_root(root, &mut apps);
    }
    apps
}
fn owner(owned: &[(PathBuf, usize)], p: &Path) -> Option<usize> {
    owned
        .iter()
        .filter(|(root, _)| p.starts_with(root))
        .max_by_key(|(root, _)| root.components().count())
        .map(|(_, i)| *i)
}
pub fn scan(extra: &[String], application: Option<&str>) -> Inventory {
    let managers_checked: Vec<String> = MANAGERS
        .iter()
        .filter(|n| which(n).is_some())
        .map(|n| (*n).into())
        .collect();
    let jobs: Vec<_> = managers_checked
        .iter()
        .map(|name| {
            let n = name.clone();
            thread::spawn(move || collect(&n))
        })
        .collect();
    let mut results = vec![];
    let mut warnings = vec![];
    for (name, job) in managers_checked.iter().zip(jobs) {
        match job.join() {
            Ok(Ok(rows)) => results.extend(rows),
            Ok(Err(e)) => warnings.push(format!("{name}: {e}")),
            Err(_) => warnings.push(format!("{name}: collector panicked")),
        }
    }
    let owned: Vec<_> = results
        .iter()
        .enumerate()
        .flat_map(|(i, r)| r.paths.iter().map(move |p| (resolved(Path::new(p)), i)))
        .collect();
    let mut extra: Vec<_> = extra.iter().map(|s| absolute(&expand(s))).collect();
    if let Some(app) = application {
        let p = absolute(&expand(app));
        if p.exists() && is_app(&p) {
            extra.push(p);
        }
    }
    for p in scan_apps(&extra) {
        let resolved = resolved(&p);
        if owned.iter().any(|(root, _)| *root == resolved) {
            continue;
        }
        let (manager, evidence) = if p.join("Contents/_MASReceipt/receipt").exists() {
            ("App Store", "应用包含 _MASReceipt/receipt")
        } else if resolved.starts_with("/System") {
            ("macOS", "系统应用目录")
        } else {
            ("Unknown", "未发现受支持的管理器记录；不代表手动安装")
        };
        results.push(Record::new(
            &p.file_stem().unwrap_or_default().to_string_lossy(),
            manager,
            evidence,
            vec![display(&p)],
        ));
    }
    let mut commands = BTreeSet::new();
    for folder in env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .filter(|p| !p.as_os_str().is_empty())
    {
        commands.extend(entries(&folder).into_iter().filter(|p| executable(p)));
    }
    if let Some(app) = application {
        let p = absolute(&expand(app));
        if p.is_file() {
            commands.insert(p);
        }
    }
    for p in commands {
        if let Some(i) = owner(&owned, &resolved(&p)) {
            let row = &mut results[i];
            let path = display(&p);
            if !row.paths.contains(&path) {
                row.paths.push(path);
            }
            let alias = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if !row.aliases.contains(&alias) {
                row.aliases.push(alias);
            }
            continue;
        }
        let system = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .iter()
            .any(|root| p.starts_with(root));
        results.push(Record::new(
            &p.file_name().unwrap_or_default().to_string_lossy(),
            if system { "macOS" } else { "Unknown" },
            if system {
                "系统命令目录"
            } else {
                "PATH 或指定路径；无已关联管理记录"
            },
            vec![display(&p)],
        ));
    }
    results.sort_by(|a, b| {
        (a.name.to_lowercase(), &a.manager, &a.paths).cmp(&(
            b.name.to_lowercase(),
            &b.manager,
            &b.paths,
        ))
    });
    Inventory {
        results,
        warnings,
        managers_checked,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ownership_uses_path_boundaries() {
        let owned = vec![
            (PathBuf::from("/tools/foo"), 0),
            (PathBuf::from("/tools/foo/bin"), 1),
        ];
        assert_eq!(owner(&owned, Path::new("/tools/foo/bin/run")), Some(1));
        assert_eq!(owner(&owned, Path::new("/tools/foobar/run")), None);
    }
    #[test]
    fn custom_directory_and_nested_apps() {
        let root = env::temp_dir().join(format!("pkg-owner-test-{}", std::process::id()));
        let app = root.join("Custom/Outside.app");
        fs::create_dir_all(app.join("Contents/Nested.app")).unwrap();
        let mut apps = BTreeSet::new();
        scan_root(&root, &mut apps);
        assert_eq!(apps, BTreeSet::from([app]));
        fs::remove_dir_all(root).unwrap();
    }
}
