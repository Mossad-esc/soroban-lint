//! Project discovery: finding the source files to analyze and the crates they
//! belong to.
//!
//! Discovery implements step 1 of the README's "How It Works": `cargo_metadata`
//! finds workspace members, crates that depend on `soroban-sdk` are treated as
//! contract crates, and files are collected with the `ignore` crate so
//! `.gitignore` is respected. When no manifest is available the given paths are
//! analyzed directly.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use ignore::overrides::{Override, OverrideBuilder};
use ignore::WalkBuilder;
use syn::File;

/// A file selected for analysis, together with its parsed syntax tree.
#[derive(Debug)]
pub struct SourceFile {
    /// The path of the source file.
    pub path: PathBuf,
    /// The name of the crate the file belongs to, when known.
    pub crate_name: Option<String>,
    /// The manifest of the crate the file belongs to, when known.
    pub manifest_path: Option<PathBuf>,
    /// The parsed syntax tree.
    pub ast: File,
}

/// A file that could not be read or parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The file that failed.
    pub path: PathBuf,
    /// 1-based line of the error, or `0` when unknown.
    pub line: usize,
    /// 1-based column of the error, or `0` when unknown.
    pub column: usize,
    /// A human-readable description of the failure.
    pub message: String,
}

/// The outcome of discovery: files that parsed and files that did not.
#[derive(Debug, Default)]
pub struct Discovery {
    /// Source files that were read and parsed successfully.
    pub files: Vec<SourceFile>,
    /// Files that could not be read or parsed.
    pub errors: Vec<ParseError>,
}

impl Discovery {
    /// Whether any file failed to read or parse.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Options controlling discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Also analyze `tests/` directories and `#[cfg(test)]` modules.
    pub include_tests: bool,
    /// Glob patterns (relative to the walk root) to skip.
    pub exclude: Vec<String>,
    /// Never access the network while reading Cargo metadata.
    pub offline: bool,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            include_tests: false,
            exclude: Vec::new(),
            offline: true,
        }
    }
}

impl DiscoveryOptions {
    /// Creates default options with the given `include_tests` value.
    #[must_use]
    pub fn with_include_tests(include_tests: bool) -> Self {
        Self {
            include_tests,
            ..Self::default()
        }
    }
}

/// Discovers and parses the source files reachable from `paths`.
///
/// The returned lists are deterministic: files and errors are sorted by path.
#[must_use]
pub fn discover(paths: &[PathBuf], options: &DiscoveryOptions) -> Discovery {
    let crate_map = CrateMap::load(paths, options);

    let mut collected: Vec<PathBuf> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();

    for path in paths {
        if path.is_file() {
            collected.push(path.clone());
        } else if path.is_dir() {
            if let Some(members) = crate_map
                .as_ref()
                .and_then(|map| map.contract_members_under(path))
            {
                for member in members {
                    collected.extend(walk_dir(&member.root_dir, options));
                }
            } else {
                collected.extend(walk_dir(path, options));
            }
        } else {
            errors.push(ParseError {
                path: path.clone(),
                line: 0,
                column: 0,
                message: "path does not exist".to_owned(),
            });
        }
    }

    collected.sort();
    collected.dedup();

    let mut files = Vec::new();
    for path in collected {
        match std::fs::read_to_string(&path) {
            Ok(source) => match syn::parse_file(&source) {
                Ok(ast) => {
                    let (crate_name, manifest_path) = crate_map
                        .as_ref()
                        .map_or((None, None), |map| map.attribute(&path));
                    files.push(SourceFile {
                        path,
                        crate_name,
                        manifest_path,
                        ast,
                    });
                }
                Err(error) => errors.push(parse_error(&path, &error)),
            },
            Err(error) => errors.push(ParseError {
                path,
                line: 0,
                column: 0,
                message: error.to_string(),
            }),
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    errors.sort_by(|a, b| {
        (a.path.clone(), a.line, a.column).cmp(&(b.path.clone(), b.line, b.column))
    });

    Discovery { files, errors }
}

/// Builds a structured parse error from a `syn` failure.
fn parse_error(path: &Path, error: &syn::Error) -> ParseError {
    let start = error.span().start();
    ParseError {
        path: path.to_path_buf(),
        line: start.line,
        column: start.column + 1,
        message: error.to_string(),
    }
}

/// Walks a directory collecting `.rs` files, honoring ignore rules.
fn walk_dir(root: &Path, options: &DiscoveryOptions) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let mut builder = OverrideBuilder::new(root);
    // Always skip build output.
    let _ = builder.add("!target/");
    if !options.include_tests {
        let _ = builder.add("!tests/");
    }
    for pattern in &options.exclude {
        let _ = builder.add(&format!("!{pattern}"));
    }
    let overrides = builder.build().unwrap_or_else(|_| Override::empty());

    let mut walk = WalkBuilder::new(root);
    walk.overrides(overrides);
    // Respect `.gitignore` even outside a git repository (for tests and
    // checked-out source trees).
    walk.require_git(false);

    for entry in walk.build() {
        let Ok(entry) = entry else { continue };
        let is_rs = entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs");
        if is_rs && entry.file_type().is_some_and(|kind| kind.is_file()) {
            files.push(entry.path().to_path_buf());
        }
    }

    files
}

/// A workspace member crate.
struct CrateMember {
    name: String,
    manifest_path: PathBuf,
    root_dir: PathBuf,
    is_contract: bool,
}

/// The workspace members discovered from Cargo metadata.
struct CrateMap {
    members: Vec<CrateMember>,
}

impl CrateMap {
    /// Loads workspace members from Cargo metadata, if possible.
    ///
    /// Returns `None` when no manifest can be found or metadata cannot be read,
    /// in which case discovery falls back to analyzing the given paths
    /// directly.
    fn load(paths: &[PathBuf], options: &DiscoveryOptions) -> Option<Self> {
        let base = paths
            .iter()
            .map(|path| {
                if path.is_dir() {
                    path.clone()
                } else {
                    path.parent()
                        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
                }
            })
            .next()?;
        let manifest = find_manifest(&base)?;

        let mut command = MetadataCommand::new();
        command.manifest_path(&manifest);
        command.no_deps();
        if options.offline {
            command.other_options(vec!["--offline".to_owned()]);
        }

        let metadata = command.exec().ok()?;
        let member_ids: HashSet<_> = metadata.workspace_members.iter().collect();

        let mut members = Vec::new();
        for package in &metadata.packages {
            if !member_ids.contains(&package.id) {
                continue;
            }
            let manifest_path = package.manifest_path.clone().into_std_path_buf();
            let root_dir = manifest_path
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let is_contract = package
                .dependencies
                .iter()
                .any(|dependency| dependency.name == "soroban-sdk");
            members.push(CrateMember {
                name: package.name.clone(),
                manifest_path,
                root_dir,
                is_contract,
            });
        }

        Some(Self { members })
    }

    /// Returns the contract members located under `dir`, if any.
    ///
    /// When the returned list is empty, the caller should walk `dir` directly.
    fn contract_members_under(&self, dir: &Path) -> Option<Vec<&CrateMember>> {
        let members: Vec<&CrateMember> = self
            .members
            .iter()
            .filter(|member| member.is_contract && member.root_dir.starts_with(dir))
            .collect();
        if members.is_empty() {
            None
        } else {
            Some(members)
        }
    }

    /// Attributes a file to the most specific member crate containing it.
    fn attribute(&self, file: &Path) -> (Option<String>, Option<PathBuf>) {
        let member = self
            .members
            .iter()
            .filter(|member| file.starts_with(&member.root_dir))
            .max_by_key(|member| member.root_dir.as_os_str().len());
        match member {
            Some(member) => (
                Some(member.name.clone()),
                Some(member.manifest_path.clone()),
            ),
            None => (None, None),
        }
    }
}

/// Walks up from `start` looking for a `Cargo.toml`.
fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn single_crate_discovers_sources() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"single\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write(&root.join("src/lib.rs"), "pub fn a() {}\n");
        write(&root.join("src/other.rs"), "pub fn b() {}\n");

        let discovery = discover(&[root.to_path_buf()], &DiscoveryOptions::default());

        assert!(!discovery.has_errors(), "{:?}", discovery.errors);
        let names: Vec<String> = discovery
            .files
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["lib.rs", "other.rs"]);
        assert_eq!(discovery.files[0].crate_name.as_deref(), Some("single"));
    }

    #[test]
    fn workspace_selects_contract_crates_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"token\", \"helper\"]\n",
        );
        write(
            &root.join("token/Cargo.toml"),
            "[package]\nname = \"token\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nsoroban-sdk = \"21\"\n",
        );
        write(&root.join("token/src/lib.rs"), "pub fn token() {}\n");
        write(
            &root.join("helper/Cargo.toml"),
            "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write(&root.join("helper/src/lib.rs"), "pub fn helper() {}\n");

        let discovery = discover(&[root.to_path_buf()], &DiscoveryOptions::default());

        assert!(!discovery.has_errors(), "{:?}", discovery.errors);
        assert_eq!(discovery.files.len(), 1);
        assert_eq!(discovery.files[0].crate_name.as_deref(), Some("token"));
    }

    #[test]
    fn gitignore_is_respected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join(".gitignore"), "ignored.rs\n");
        write(&root.join("src/kept.rs"), "pub fn kept() {}\n");
        write(&root.join("src/ignored.rs"), "pub fn ignored() {}\n");

        let discovery = discover(&[root.to_path_buf()], &DiscoveryOptions::default());

        assert!(!discovery.has_errors(), "{:?}", discovery.errors);
        let names: Vec<String> = discovery
            .files
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["kept.rs"]);
    }

    #[test]
    fn tests_directory_skipped_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("src/lib.rs"), "pub fn a() {}\n");
        write(&root.join("tests/integration.rs"), "pub fn b() {}\n");

        let without = discover(&[root.to_path_buf()], &DiscoveryOptions::default());
        let names: Vec<String> = without
            .files
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["lib.rs"]);

        let with = discover(
            &[root.to_path_buf()],
            &DiscoveryOptions::with_include_tests(true),
        );
        let names: Vec<String> = with
            .files
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["lib.rs", "integration.rs"]);
    }

    #[test]
    fn syntax_error_is_reported_but_other_files_are_analyzed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("good.rs"), "pub fn good() {}\n");
        write(&root.join("bad.rs"), "pub fn bad( {\n");

        let discovery = discover(&[root.to_path_buf()], &DiscoveryOptions::default());

        assert!(discovery.has_errors());
        assert_eq!(discovery.files.len(), 1);
        assert_eq!(discovery.files[0].path.file_name().unwrap(), "good.rs");
        assert_eq!(discovery.errors[0].path.file_name().unwrap(), "bad.rs");
    }

    #[test]
    fn missing_path_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let discovery = discover(&[missing], &DiscoveryOptions::default());
        assert!(discovery.has_errors());
        assert!(discovery.files.is_empty());
    }
}
