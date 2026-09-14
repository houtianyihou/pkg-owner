pub mod collectors;
pub mod inventory;
pub mod process;

use serde::Serialize;
use unicode_casefold::UnicodeCaseFold;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Record {
    pub name: String,
    pub manager: String,
    pub evidence: String,
    pub paths: Vec<String>,
    pub aliases: Vec<String>,
}
impl Record {
    pub fn new(name: &str, manager: &str, evidence: &str, paths: Vec<String>) -> Self {
        Self {
            name: name.into(),
            manager: manager.into(),
            evidence: evidence.into(),
            paths,
            aliases: vec![],
        }
    }
}

pub fn normalize(s: &str) -> String {
    s.case_fold().filter(|c| c.is_alphanumeric()).collect()
}

// Ratcliff/Obershelp matching, same longest-block tie order as SequenceMatcher.
fn matching_chars(a: &[char], b: &[char]) -> usize {
    let (mut ai, mut bi, mut best) = (0, 0, 0);
    let mut previous = vec![0; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        let mut current = vec![0; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            if x == y {
                current[j + 1] = previous[j] + 1;
                if current[j + 1] > best {
                    best = current[j + 1];
                    ai = i + 1 - best;
                    bi = j + 1 - best;
                }
            }
        }
        previous = current;
    }
    if best == 0 {
        return 0;
    }
    best + matching_chars(&a[..ai], &b[..bi]) + matching_chars(&a[ai + best..], &b[bi + best..])
}
pub fn similar(a: &str, b: &str) -> bool {
    let a: Vec<_> = a.chars().collect();
    let b: Vec<_> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return true;
    }
    2.0 * matching_chars(&a, &b) as f64 / (a.len() + b.len()) as f64 >= 0.72
}
pub fn matches(row: &Record, query: &str, fuzzy: bool) -> bool {
    let q = normalize(query);
    let mut values = vec![row.name.clone()];
    values.extend(row.aliases.clone());
    values.extend(row.paths.clone());
    values.extend(row.paths.iter().filter_map(|p| {
        std::path::Path::new(p)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
    }));
    if fuzzy {
        let words: Vec<String> = values
            .iter()
            .flat_map(|s| {
                s.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .collect();
        values.extend(words);
    }
    values.iter().any(|v| {
        let v = normalize(v);
        q == v || (fuzzy && (v.contains(&q) || similar(&q, &v)))
    })
}
fn escape(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() {
                format!("\\x{:02x}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect()
}
pub fn format_table(rows: &[Record]) -> String {
    let mut cells = vec![["名称".into(), "管理来源".into(), "路径 / 证据".into()]];
    cells.extend(rows.iter().map(|r| {
        [
            escape(&r.name),
            escape(&r.manager),
            escape(&if r.paths.is_empty() {
                r.evidence.clone()
            } else {
                r.paths
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
        ]
    }));
    let widths: Vec<_> = (0..2)
        .map(|i| cells.iter().map(|r| r[i].width()).max().unwrap_or(0))
        .collect();
    cells
        .iter()
        .map(|r| {
            format!(
                "{}{}  {}{}  {}",
                r[0],
                " ".repeat(widths[0] - r[0].width()),
                r[1],
                " ".repeat(widths[1] - r[1].width()),
                r[2]
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuzzy_and_unicode() {
        let r = Record::new("Google Chrome", "Unknown", "", vec![]);
        assert!(matches(&r, "chorme", true));
        assert!(!matches(&r, "firefox", true));
        assert!(!matches(&r, "chrome", false));
        assert_eq!(normalize("Straße"), "strasse");
    }
    #[test]
    fn columns_and_controls() {
        let rows = vec![
            Record::new("中文应用", "macOS", "MARK", vec![]),
            Record::new("Claude Code URL Handler", "Unknown", "MARK", vec![]),
            Record::new("e\u{301}\n", "Unknown", "MARK", vec![]),
        ];
        let table = format_table(&rows);
        let widths: Vec<_> = table
            .lines()
            .skip(1)
            .map(|l| l.split("MARK").next().unwrap().width())
            .collect();
        assert!(widths.iter().all(|w| *w == widths[0]));
        assert_eq!(table.lines().count(), 4);
        assert!(format_table(&[]).contains("名称"));
    }
}
