//! Tests that each rule docs page has the required sections and markers.
//! The full correctness of examples is verified by fixture snapshot tests.

fn workspace_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

const RULE_DOCS: &[(&str, &str, &str)] = &[
    ("SOR001", "SOR001.md", "sor001"),
    ("SOR002", "SOR002.md", "sor002"),
    ("SOR003", "SOR003.md", "sor003"),
    ("SOR004", "SOR004.md", "sor004"),
    ("SOR005", "SOR005.md", "sor005"),
    ("SOR006", "SOR006.md", "sor006"),
    ("SOR007", "SOR007.md", "sor007"),
    ("SOR008", "SOR008.md", "sor008"),
];

/// Every docs page must exist.
#[test]
fn all_docs_pages_exist() {
    let root = workspace_root();
    let mut missing = Vec::new();
    for (_id, filename, _prefix) in RULE_DOCS {
        let path = root.join("docs").join("rules").join(filename);
        if !path.is_file() {
            missing.push(path.display().to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "missing docs pages:\n{}",
        missing.join("\n")
    );
}

/// Every docs page must have at least one `vulnerable` and one `safe` marker.
#[test]
fn docs_pages_have_example_markers() {
    let root = workspace_root();
    let mut errors = Vec::new();
    for (_id, filename, _prefix) in RULE_DOCS {
        let path = root.join("docs").join("rules").join(filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{filename}: cannot read: {e}"));
                continue;
            }
        };
        if !content.contains("soroban-lint: vulnerable")
            && !content.contains("Vulnerable example")
            && !content.contains("## Vulnerable")
        {
            errors.push(format!("{filename}: missing vulnerable example section"));
        }
        if !content.contains("soroban-lint: safe")
            && !content.contains("Safe example")
            && !content.contains("## Safe")
        {
            errors.push(format!("{filename}: missing safe example section"));
        }
    }
    assert!(
        errors.is_empty(),
        "docs pages have missing sections:\n{}",
        errors.join("\n")
    );
}

/// Each implemented rule must have a fixture whose name starts with the rule's
/// lower-case prefix, confirming the docs examples are tied to real fixtures.
#[test]
fn fixture_files_exist_for_each_rule() {
    let root = workspace_root();
    let mut missing = Vec::new();
    for (_id, _filename, prefix) in RULE_DOCS {
        for subdir in &["vulnerable", "safe"] {
            let dir = root.join("tests").join("fixtures").join(subdir);
            let has_fixture = std::fs::read_dir(&dir)
                .map(|entries| {
                    entries.flatten().any(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .to_lowercase()
                            .starts_with(prefix)
                    })
                })
                .unwrap_or(false);
            if !has_fixture {
                missing.push(format!("tests/fixtures/{subdir}/{prefix}*.rs"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing fixture files:\n{}",
        missing.join("\n")
    );
}
