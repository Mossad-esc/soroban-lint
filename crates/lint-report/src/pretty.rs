//! The `pretty` terminal reporter.
//!
//! Renders compiler-style diagnostics matching the README's "Quick Start"
//! sample output, using `annotate-snippets` for the source-snippet portion.
//!
//! ## Output structure (per finding)
//!
//! ```text
//! error[SOR001]: entrypoint `transfer` writes to storage but never calls `require_auth`
//!   --> contracts/token/src/lib.rs:42:12
//!    |
//! 42 |     pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
//!    |            ^^^^^^^^
//!    |
//!    = help: call `from.require_auth()` before mutating state
//!    = docs: https://…/rules/SOR001
//! ```
//!
//! ## Summary line
//!
//! ```text
//! Found 1 error, 1 warning in 2 files (0.04s)
//! ```
//!
//! or, when there are no findings:
//!
//! ```text
//! No issues found in 3 files (0.01s)
//! ```
//!
//! ## Color
//!
//! Color is controlled by the `use_color` flag passed to [`PrettyReporter`].
//! The CLI sets this based on `--no-color`, `NO_COLOR`, and TTY detection.

use std::fmt::Write as FmtWrite;
use std::path::Path;

use annotate_snippets::{Level, Renderer, Snippet};

use lint_core::{Finding, Severity};

use crate::{Report, Reporter};

/// Pretty terminal reporter.
///
/// Construct with [`PrettyReporter::new`], passing whether colour output
/// should be used.
#[derive(Debug, Clone)]
pub struct PrettyReporter {
    use_color: bool,
}

impl PrettyReporter {
    /// Creates a new pretty reporter.
    ///
    /// `use_color` controls ANSI colour codes.  Pass `false` in tests and when
    /// `--no-color` / `NO_COLOR` / non-TTY is detected.
    #[must_use]
    pub fn new(use_color: bool) -> Self {
        Self { use_color }
    }
}

impl Reporter for PrettyReporter {
    fn render(&self, report: &Report) -> Result<String, String> {
        let mut out = String::new();

        for finding in &report.findings {
            let block = render_finding(finding, self.use_color);
            out.push_str(&block);
            out.push('\n');
        }

        out.push_str(&render_summary(report));
        out.push('\n');

        Ok(out)
    }
}

/// Renders a single finding as a compiler-style diagnostic block.
fn render_finding(finding: &Finding, use_color: bool) -> String {
    // Read the source file for the snippet; if it cannot be read, fall back to
    // a no-snippet rendering (file/line/col only).
    let source_text = std::fs::read_to_string(&finding.file).ok();

    let file_display = display_path(&finding.file);

    // Build the `annotate-snippets` level.
    let level = match finding.severity {
        Severity::Error => Level::Error,
        Severity::Warning => Level::Warning,
        Severity::Note => Level::Note,
    };

    // Build the title: "entrypoint `X` writes to storage…"
    let title = finding.message.as_str();

    // Build the `-->` origin string: just the file path; annotate-snippets
    // will append the line number itself from the snippet's line_start +
    // annotation offset.
    let origin = file_display.clone();

    let renderer = if use_color {
        Renderer::styled()
    } else {
        Renderer::plain()
    };

    if let Some(ref source) = source_text {
        // Locate the byte range of the identifier on the reported line so we
        // can underline it.  `annotate-snippets` works with byte ranges into
        // the source string.
        let (start_byte, end_byte) = ident_byte_range(source, finding.line, finding.column);

        // Extract the single source line for the snippet.
        let snippet_source = source.as_str();

        let mut message = level.title(title).id(&finding.rule_id).snippet(
            Snippet::source(snippet_source)
                .line_start(1)
                .origin(&origin)
                .fold(true)
                .annotation(level.span(start_byte..end_byte)),
        );

        // Attach `= help:` footer.
        if let Some(ref help) = finding.help {
            message = message.footer(Level::Help.title(help));
        }

        // Attach `= note: docs: ...` footer.
        if let Some(ref url) = finding.docs_url {
            let docs_label = format!("docs: {url}");
            // We need an owned string for the footer title; we heap-allocate
            // it here and leak it into a &'static str via a Box.  This is
            // acceptable because findings are short-lived and the number of
            // findings is bounded.
            let label: &'static str = Box::leak(docs_label.into_boxed_str());
            message = message.footer(Level::Note.title(label));
        }

        renderer.render(message).to_string()
    } else {
        // No source available: render a minimal diagnostic without a snippet.
        let mut message = level.title(title).id(&finding.rule_id);

        if let Some(ref help) = finding.help {
            message = message.footer(Level::Help.title(help));
        }
        if let Some(ref url) = finding.docs_url {
            let docs_label = format!("docs: {url}");
            let label: &'static str = Box::leak(docs_label.into_boxed_str());
            message = message.footer(Level::Note.title(label));
        }

        let mut out = renderer.render(message).to_string();
        // Append the location manually since there is no snippet.
        let manual_origin = format!("{}:{}:{}", file_display, finding.line, finding.column);
        writeln!(out, "  --> {manual_origin}").ok();
        out
    }
}

/// Returns the `(start_byte, end_byte)` range of the identifier starting at
/// the given 1-based `line`/`column` in `source`.
///
/// The range spans the identifier token (consecutive alphanumeric + `_`
/// characters).  If the location is out of range the function returns a
/// zero-length range at the computed start byte, so `annotate-snippets` still
/// renders the `-->` pointer without an underline.
fn ident_byte_range(source: &str, line: usize, column: usize) -> (usize, usize) {
    // Walk to the start of the target line.
    let mut current_line = 1usize;
    let mut line_start_byte = 0usize;
    for (byte_idx, ch) in source.char_indices() {
        if current_line == line {
            line_start_byte = byte_idx;
            break;
        }
        if ch == '\n' {
            current_line += 1;
        }
    }
    // Handle the case where `line` is the last line and has no trailing newline.
    if current_line < line {
        // Line not found; return a zero-length range at end of source.
        let end = source.len();
        return (end, end);
    }

    // `column` is 1-based.  Walk `column - 1` characters from the line start.
    let col_offset = column.saturating_sub(1);
    let mut start_byte = line_start_byte;
    for (col_char, (byte_offset, _ch)) in source[line_start_byte..].char_indices().enumerate() {
        if col_char == col_offset {
            start_byte = line_start_byte + byte_offset;
            break;
        }
    }

    // Extend over the identifier characters.
    let end_byte = source[start_byte..]
        .char_indices()
        .take_while(|(_, ch)| ch.is_alphanumeric() || *ch == '_')
        .last()
        .map(|(byte_offset, ch)| start_byte + byte_offset + ch.len_utf8())
        .unwrap_or(start_byte);

    (start_byte, end_byte.max(start_byte + 1))
}

/// Renders the summary line.
///
/// Examples:
/// - `No issues found in 3 files (0.01s)`
/// - `Found 1 error in 1 file (0.02s)`
/// - `Found 2 errors, 1 warning in 4 files (0.05s)`
fn render_summary(report: &Report) -> String {
    let errors = report.error_count();
    let warnings = report.warning_count();
    let notes = report.note_count();
    let files = report.files_scanned;
    let secs = report.elapsed.as_secs_f64();

    let file_str = if files == 1 { "file" } else { "files" };

    let baseline_suffix = if report.baselined_count > 0 {
        format!(" ({} baselined)", report.baselined_count)
    } else {
        String::new()
    };

    if errors == 0 && warnings == 0 && notes == 0 {
        return format!("No issues found in {files} {file_str} ({secs:.2}s){baseline_suffix}");
    }

    let mut parts: Vec<String> = Vec::new();
    if errors > 0 {
        parts.push(format!("{errors} {}", pluralise(errors, "error", "errors")));
    }
    if warnings > 0 {
        parts.push(format!(
            "{warnings} {}",
            pluralise(warnings, "warning", "warnings")
        ));
    }
    if notes > 0 {
        parts.push(format!("{notes} {}", pluralise(notes, "note", "notes")));
    }

    format!(
        "Found {} in {files} {file_str} ({secs:.2}s){baseline_suffix}",
        parts.join(", ")
    )
}

/// Returns `singular` when `count == 1`, otherwise `plural`.
fn pluralise<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Renders a file path as a forward-slash string relative to the current
/// working directory, falling back to the absolute path.
fn display_path(path: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    path.strip_prefix(&cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::Finding;
    use std::path::PathBuf;
    use std::time::Duration;

    fn make_finding(rule_id: &str, severity: Severity, line: usize, col: usize) -> Finding {
        Finding::new(
            rule_id,
            "missing-require-auth",
            severity,
            "entrypoint `transfer` writes to storage but never calls `require_auth`",
            PathBuf::from("tests/fixtures/vulnerable/sor001_basic.rs"),
            line,
            col,
        )
        .with_help("call `from.require_auth()` before mutating state")
        .with_docs_url("https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR001.md")
    }

    fn fixed_report(findings: Vec<Finding>, files: usize) -> Report {
        Report {
            findings,
            files_scanned: files,
            elapsed: Duration::from_millis(42),
            baselined_count: 0,
        }
    }

    #[test]
    fn summary_no_findings() {
        let report = fixed_report(vec![], 3);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(output.contains("No issues found in 3 files"));
        assert!(output.contains("0.04s"));
    }

    #[test]
    fn summary_one_error() {
        let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 1, 1)], 1);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(output.contains("Found 1 error in 1 file"));
    }

    #[test]
    fn summary_plural_errors_and_warnings() {
        let findings = vec![
            make_finding("SOR001", Severity::Error, 1, 1),
            make_finding("SOR001", Severity::Error, 2, 1),
            make_finding("SOR002", Severity::Warning, 3, 1),
        ];
        let report = fixed_report(findings, 2);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(output.contains("Found 2 errors, 1 warning in 2 files"));
    }

    #[test]
    fn summary_one_file() {
        let report = fixed_report(vec![], 1);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(output.contains("in 1 file"));
        assert!(!output.contains("in 1 files"));
    }

    #[test]
    fn output_contains_rule_id_and_message() {
        let finding = make_finding("SOR001", Severity::Error, 23, 12);
        let report = fixed_report(vec![finding], 1);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(output.contains("SOR001"));
        assert!(output.contains("transfer"));
        assert!(output.contains("require_auth"));
    }

    #[test]
    fn output_contains_help_and_docs() {
        let finding = make_finding("SOR001", Severity::Error, 23, 12);
        let report = fixed_report(vec![finding], 1);
        let output = PrettyReporter::new(false).render(&report).unwrap();
        assert!(
            output.contains("call `from.require_auth()`"),
            "help line missing"
        );
        assert!(output.contains("docs: https://"), "docs line missing");
    }

    #[test]
    fn ident_byte_range_finds_function_name() {
        let source = "pub fn transfer(env: Env) {}\n";
        // "transfer" starts at column 8 (1-based), line 1.
        let col = source.find("transfer").unwrap() + 1; // 1-based
        let (start, end) = ident_byte_range(source, 1, col);
        assert_eq!(&source[start..end], "transfer");
    }

    #[test]
    fn ident_byte_range_handles_out_of_range() {
        let source = "fn f() {}";
        // Line 99 doesn't exist — should not panic, returns end..end.
        let (s, e) = ident_byte_range(source, 99, 1);
        assert_eq!(s, e);
    }

    #[test]
    fn pluralise_singular_and_plural() {
        assert_eq!(pluralise(1, "error", "errors"), "error");
        assert_eq!(pluralise(2, "error", "errors"), "errors");
        assert_eq!(pluralise(0, "error", "errors"), "errors");
    }
}
