//! Maintainer tasks for the NoteCraft workspace.
//!
//! This project is deliberately Rust-only. Build, test, and maintenance
//! automation all live here so there is a single implementation language and
//! no shell scripts, batch files, or other-language tooling to maintain.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The crate that composes the application (the composition root).
pub const APP_CRATE: &str = "nc-app";
/// The crate that owns all native/OS integration.
pub const PLATFORM_CRATE: &str = "nc-platform";
/// Crates that must stay headless: no dependency on the platform or app crates,
/// so they build and test on any host.
pub const HEADLESS_CRATES: &[&str] = &["nc-commands", "nc-markdown"];

/// A crate discovered under `<root>/crates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Absolute path to the crate directory.
    pub dir: PathBuf,
    /// The `[package] name` of the crate.
    pub name: String,
    /// Names of the crates this one depends on (any dependency kind).
    pub dependencies: BTreeSet<String>,
}

/// Errors surfaced while inspecting the workspace.
#[derive(Debug)]
pub enum Error {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    MissingPackageName {
        path: PathBuf,
    },
    WorkspaceRootNotFound {
        start: PathBuf,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::MissingPackageName { path } => {
                write!(f, "{}: no [package] name found", path.display())
            }
            Error::WorkspaceRootNotFound { start } => write!(
                f,
                "could not locate the workspace root (a Cargo.toml containing [workspace]) \
                 starting from {}",
                start.display()
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Walk up from `start` looking for the workspace root manifest.
pub fn find_workspace_root(start: &Path) -> Result<PathBuf, Error> {
    let mut dir = Some(start);
    while let Some(current) = dir {
        let manifest = current.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = fs::read_to_string(&manifest) {
                if text.contains("[workspace]") {
                    return Ok(current.to_path_buf());
                }
            }
        }
        dir = current.parent();
    }
    Err(Error::WorkspaceRootNotFound {
        start: start.to_path_buf(),
    })
}

/// Discover every crate under `<root>/crates`, sorted by directory.
pub fn discover(root: &Path) -> Result<Vec<Member>, Error> {
    let crates_dir = root.join("crates");
    let entries = fs::read_dir(&crates_dir).map_err(|source| Error::Io {
        path: crates_dir.clone(),
        source,
    })?;

    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: crates_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();

    let mut members = Vec::new();
    for dir in dirs {
        let manifest_path = dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = fs::read_to_string(&manifest_path).map_err(|source| Error::Io {
            path: manifest_path.clone(),
            source,
        })?;
        let name = package_name(&manifest).ok_or_else(|| Error::MissingPackageName {
            path: manifest_path.clone(),
        })?;
        members.push(Member {
            dir,
            name,
            dependencies: declared_dependencies(&manifest),
        });
    }
    Ok(members)
}

/// Verify the crate dependency direction. Returns the list of violations;
/// an empty list means the workspace is clean.
pub fn check_boundaries(root: &Path) -> Result<Vec<String>, Error> {
    let members = discover(root)?;
    let mut violations = Vec::new();

    // Rule 1: the platform crate must never depend on the app crate.
    if let Some(platform) = members.iter().find(|m| m.name == PLATFORM_CRATE) {
        if platform.dependencies.contains(APP_CRATE) {
            violations.push(format!(
                "{PLATFORM_CRATE} ({}) must not depend on {APP_CRATE}",
                relative(root, &platform.dir)
            ));
        }
    }

    // Rule 2: only the app crate (the composition root) may depend on the
    // platform crate.
    for member in &members {
        if member.name == APP_CRATE {
            continue;
        }
        if member.dependencies.contains(PLATFORM_CRATE) {
            violations.push(format!(
                "only {APP_CRATE} may depend on {PLATFORM_CRATE}; {} ({}) does",
                member.name,
                relative(root, &member.dir)
            ));
        }
    }

    // Rule 3: headless crates must never reach up to the shell or the native
    // layer — that is what keeps them testable on a host without Windows.
    for member in &members {
        if !HEADLESS_CRATES.contains(&member.name.as_str()) {
            continue;
        }
        for forbidden in [PLATFORM_CRATE, APP_CRATE] {
            if member.dependencies.contains(forbidden) {
                violations.push(format!(
                    "{} ({}) is headless and must not depend on {forbidden}",
                    member.name,
                    relative(root, &member.dir)
                ));
            }
        }
    }

    Ok(violations)
}

/// Extract the `[package] name` value from a manifest.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(value) = line.strip_prefix("name") {
            if let Some((_, value)) = value.split_once('=') {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Collect the crate names declared in the manifest's dependency tables.
fn declared_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let mut in_deps = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_deps = line.starts_with("[dependencies")
                || line.starts_with("[dev-dependencies")
                || line.starts_with("[build-dependencies");
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            // `nc-platform.workspace = true` -> `nc-platform`
            let key = key.trim().trim_matches('"');
            let name = key.split('.').next().unwrap_or(key).trim();
            if !name.is_empty() {
                deps.insert(name.to_string());
            }
        }
    }
    deps
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_name() {
        let manifest = "[package]\nname = \"nc-app\"\nversion = \"0.0.0\"\n";
        assert_eq!(package_name(manifest).as_deref(), Some("nc-app"));
    }

    #[test]
    fn collects_dependency_keys_across_tables() {
        let manifest = "\
[package]
name = \"demo\"

[dependencies]
nc-platform.workspace = true
anyhow = \"1.0\"

[dev-dependencies]
tempfile = \"3\"
";
        let deps = declared_dependencies(manifest);
        assert!(deps.contains("nc-platform"));
        assert!(deps.contains("anyhow"));
        assert!(deps.contains("tempfile"));
        assert!(!deps.contains("demo"));
    }

    #[test]
    fn dependency_scan_ignores_docs_and_other_sections() {
        let manifest = "\
[package]
name = \"demo\"

[dependencies]
serde = { version = \"1\" }
";
        let deps = declared_dependencies(manifest);
        assert_eq!(deps.len(), 1);
        assert!(deps.contains("serde"));
    }

    fn scratch_workspace(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("notecraft-xtask-test-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates")).unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        root
    }

    fn crate_at(root: &Path, dir: &str, name: &str, deps: &[&str]) {
        let crate_dir = root.join("crates").join(dir);
        fs::create_dir_all(&crate_dir).unwrap();
        let mut manifest = format!("[package]\nname = \"{name}\"\n");
        if !deps.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for dep in deps {
                manifest.push_str(&format!("{dep}.workspace = true\n"));
            }
        }
        fs::write(crate_dir.join("Cargo.toml"), manifest).unwrap();
    }

    #[test]
    fn flags_dependency_direction_violations() {
        let root = scratch_workspace("violations");
        crate_at(&root, "app", "nc-app", &["nc-platform"]);
        crate_at(&root, "platform", "nc-platform", &["nc-app"]);
        crate_at(&root, "core", "nc-core", &["nc-platform"]);

        let violations = check_boundaries(&root).unwrap();
        assert_eq!(
            violations.len(),
            2,
            "expected platform->app and core->platform violations: {violations:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clean_workspace_passes() {
        let root = scratch_workspace("clean");
        crate_at(&root, "app", "nc-app", &["nc-platform"]);
        crate_at(&root, "platform", "nc-platform", &[]);

        let violations = check_boundaries(&root).unwrap();
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn flags_headless_crate_reaching_up() {
        let root = scratch_workspace("headless");
        crate_at(&root, "commands", "nc-commands", &["nc-platform"]);

        let violations = check_boundaries(&root).unwrap();
        // Both rules catch it: reaching a headless crate up to the native layer
        // also breaks the "only the app may depend on the platform" rule.
        assert_eq!(
            violations.len(),
            2,
            "expected nc-commands->nc-platform to be flagged twice: {violations:?}"
        );
        assert!(violations.iter().any(|v| v.contains("headless")));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn headless_crate_without_native_dependencies_passes() {
        let root = scratch_workspace("headless-clean");
        crate_at(&root, "commands", "nc-commands", &[]);
        crate_at(&root, "app", "nc-app", &["nc-platform"]);
        crate_at(&root, "platform", "nc-platform", &["nc-commands"]);

        let violations = check_boundaries(&root).unwrap();
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    // --- `docs/feature-matrix.md` consistency --------------------------------

    use std::collections::BTreeMap;

    /// The status values the matrix documents (`AGENT.md` §11) and the only
    /// values a row's `Impl` column may carry.
    const STATUS_VALUES: &[&str] = &[
        "NOT_STARTED",
        "SPECIFIED",
        "IN_PROGRESS",
        "IMPLEMENTED",
        "VERIFIED",
        "BLOCKED",
        "OUT_OF_SCOPE",
    ];

    /// The `(id, Impl status)` of every `| NC-… |` row, in file order.
    fn feature_rows(text: &str) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if !line.starts_with("| NC-") {
                continue;
            }
            // "", id, feature, priority, milestone, spec, impl, …, ""
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if let (Some(id), Some(impl_status)) = (cells.get(1), cells.get(6)) {
                rows.push((id.to_string(), impl_status.to_string()));
            }
        }
        rows
    }

    /// The verification-summary table's numbers, keyed by the left-hand cell:
    /// the `Total tracked features` row and each per-status count.
    fn summary_counts(text: &str) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for raw in text.lines() {
            let line = raw.trim();
            if !line.starts_with('|') {
                continue;
            }
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() != 4 {
                continue;
            }
            let Ok(value) = cells[2].parse::<usize>() else {
                continue;
            };
            if cells[1] == "Total tracked features" || STATUS_VALUES.contains(&cells[1]) {
                counts.insert(cells[1].to_string(), value);
            }
        }
        counts
    }

    /// The number word the verification summary spells out ("Fifty-one"), if
    /// its sentence is present.
    fn prose_tally(text: &str) -> Option<&str> {
        let marker = "**What `VERIFIED` means here.** ";
        let rest = &text[text.find(marker)? + marker.len()..];
        let end = rest.find(" rows are verified")?;
        Some(rest[..end].trim())
    }

    /// The English words for a count, matching the prose's style (`1` → `One`,
    /// `51` → `Fifty-one`).
    fn count_words(n: usize) -> String {
        const ONES: [&str; 20] = [
            "Zero",
            "One",
            "Two",
            "Three",
            "Four",
            "Five",
            "Six",
            "Seven",
            "Eight",
            "Nine",
            "Ten",
            "Eleven",
            "Twelve",
            "Thirteen",
            "Fourteen",
            "Fifteen",
            "Sixteen",
            "Seventeen",
            "Eighteen",
            "Nineteen",
        ];
        const TENS: [&str; 10] = [
            "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
        ];
        if n < 20 {
            return ONES[n].to_string();
        }
        if n < 100 {
            return match n % 10 {
                0 => TENS[n / 10].to_string(),
                // Only the first word of the phrase is capitalised (`Fifty-one`,
                // not `Fifty-One`).
                ones => format!("{}-{}", TENS[n / 10], ONES[ones].to_ascii_lowercase()),
            };
        }
        match n % 100 {
            0 => format!("{} hundred", ONES[n / 100]),
            rest => format!(
                "{} hundred and {}",
                ONES[n / 100],
                count_words(rest).to_ascii_lowercase()
            ),
        }
    }

    /// Every way the matrix disagrees with the rows it summarises. Empty means
    /// it is consistent. This is the machine half of §11 ("use these states
    /// consistently") and of §2's rule that a feature is not `VERIFIED` by
    /// assertion: the counts and the spelled-out tally a reader trusts are
    /// checked against the rows, so a hand-kept total cannot quietly drift the
    /// way it once did.
    fn feature_matrix_violations(text: &str) -> Vec<String> {
        let rows = feature_rows(text);
        if rows.is_empty() {
            return vec!["no `| NC-… |` feature rows found".to_string()];
        }

        let mut violations = Vec::new();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (id, impl_status) in &rows {
            if !seen.insert(id) {
                violations.push(format!("{id} appears more than once"));
            }
            if !STATUS_VALUES.contains(&impl_status.as_str()) {
                violations.push(format!("{id} has the unknown Impl status `{impl_status}`"));
            }
            *counts.entry(impl_status.as_str()).or_default() += 1;
        }

        let summary = summary_counts(text);
        match summary.get("Total tracked features").copied() {
            Some(total) if total == rows.len() => {}
            Some(total) => violations.push(format!(
                "the summary says {total} tracked features but the rows number {}",
                rows.len()
            )),
            None => violations.push("the summary has no `Total tracked features` row".to_string()),
        }

        // A status used by a row and one counted by the summary are the same
        // set, with the same number on each side.
        let mut statuses: BTreeSet<&str> = counts.keys().copied().collect();
        statuses.extend(
            summary
                .keys()
                .map(String::as_str)
                .filter(|key| *key != "Total tracked features"),
        );
        for status in statuses {
            let from_rows = counts.get(status).copied().unwrap_or(0);
            match summary.get(status).copied() {
                Some(reported) if reported == from_rows => {}
                Some(reported) => violations.push(format!(
                    "the summary says {reported} `{status}` but the rows number {from_rows}"
                )),
                None => violations.push(format!(
                    "the rows include {from_rows} `{status}` but the summary does not count it"
                )),
            }
        }

        let verified = counts.get("VERIFIED").copied().unwrap_or(0);
        match prose_tally(text) {
            Some(words) if words == count_words(verified) => {}
            Some(words) => violations.push(format!(
                "the verification summary says `{words}` verified but the rows number \
                 {verified} (`{}`)",
                count_words(verified)
            )),
            None => {
                violations.push("the verification-summary prose tally was not found".to_string())
            }
        }

        violations
    }

    fn feature_matrix_text() -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/feature-matrix.md");
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
    }

    #[test]
    fn the_feature_matrix_summary_matches_its_rows() {
        let violations = feature_matrix_violations(&feature_matrix_text());
        assert!(
            violations.is_empty(),
            "docs/feature-matrix.md is internally inconsistent:\n  {}",
            violations.join("\n  ")
        );
    }

    #[test]
    fn the_feature_matrix_guard_catches_drifted_counts() {
        // Each mutation is what a hand-edited total or a mistyped status looks
        // like, so a green run means the guard is actually reading the rows.
        let text = feature_matrix_text();

        let wrong_total = text.replace(
            "| Total tracked features | 68 |",
            "| Total tracked features | 67 |",
        );
        assert!(
            feature_matrix_violations(&wrong_total)
                .iter()
                .any(|v| v.contains("tracked features")),
            "a wrong total must be reported"
        );

        let wrong_status = text.replace("| VERIFIED | 51 |", "| VERIFIED | 52 |");
        assert!(
            feature_matrix_violations(&wrong_status)
                .iter()
                .any(|v| v.contains("`VERIFIED`")),
            "a wrong per-status count must be reported"
        );

        let wrong_words =
            text.replace("Fifty-one rows are verified", "Fifty-two rows are verified");
        assert!(
            feature_matrix_violations(&wrong_words)
                .iter()
                .any(|v| v.contains("verified")),
            "a wrong prose tally must be reported"
        );

        // A single row's status flipped out of the counted set is caught by the
        // per-status comparison, and a typo is caught as an unknown status.
        let bad_status = text.replace(
            "| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | VERIFIED",
            "| NC-F001 | New empty document (Ctrl+N) | MANDATORY | M4 | SPECIFIED | VERIFED",
        );
        assert!(
            feature_matrix_violations(&bad_status)
                .iter()
                .any(|v| v.contains("unknown Impl status")),
            "a mistyped status must be reported"
        );
    }

    #[test]
    fn parses_matrix_rows_and_summary() {
        let text = "\
| ID | Feature | Priority | Milestone | Spec | Impl | Acceptance | Test |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NC-F001 | A thing | MANDATORY | M4 | SPECIFIED | VERIFIED | does a thing | `a.rs` |
| NC-E004 | Another | MANDATORY | M3 | SPECIFIED | IMPLEMENTED | partly | `b.rs` |

| Metric | Value |
| --- | --- |
| Total tracked features | 2 |
| VERIFIED | 1 |
";
        assert_eq!(
            feature_rows(text),
            vec![
                ("NC-F001".to_string(), "VERIFIED".to_string()),
                ("NC-E004".to_string(), "IMPLEMENTED".to_string()),
            ]
        );
        let summary = summary_counts(text);
        assert_eq!(summary.get("Total tracked features"), Some(&2));
        assert_eq!(summary.get("VERIFIED"), Some(&1));
        // A row count with no summary count is a violation.
        assert!(
            feature_matrix_violations(text)
                .iter()
                .any(|v| v.contains("`IMPLEMENTED`"))
        );
    }

    #[test]
    fn spells_counts_the_way_the_summary_does() {
        assert_eq!(count_words(0), "Zero");
        assert_eq!(count_words(1), "One");
        assert_eq!(count_words(20), "Twenty");
        assert_eq!(count_words(51), "Fifty-one");
        assert_eq!(count_words(68), "Sixty-eight");
        assert_eq!(count_words(100), "One hundred");
        assert_eq!(count_words(115), "One hundred and fifteen");
    }
}
