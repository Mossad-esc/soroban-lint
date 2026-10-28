//! Reporters (`pretty`, JSON, SARIF) for `soroban-lint`.

pub mod json;
pub mod pretty;
pub mod sarif;

use std::time::Duration;

use lint_core::Finding;

/// A completed analysis run ready to be reported.
#[derive(Debug)]
pub struct Report {
    /// All findings after filtering (min-severity, --select, --ignore,
    /// suppressions).  Must be sorted before building the report if the output
    /// should be deterministic.
    pub findings: Vec<Finding>,
    /// The number of source files that were analysed (successfully or not).
    pub files_scanned: usize,
    /// How long the run took.  Set to a fixed value in tests so snapshots are
    /// stable.
    pub elapsed: Duration,
    /// Number of findings suppressed by a baseline file (0 when no baseline).
    pub baselined_count: usize,
}

impl Report {
    /// Counts findings at `Error` severity.
    #[must_use]
    pub fn error_count(&self) -> usize {
        use lint_core::Severity;
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    /// Counts findings at `Warning` severity.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        use lint_core::Severity;
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }

    /// Counts findings at `Note` severity.
    #[must_use]
    pub fn note_count(&self) -> usize {
        use lint_core::Severity;
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Note)
            .count()
    }
}

/// Renders a [`Report`] to a `String`.
///
/// Implementors must produce deterministic output when the findings slice is
/// sorted, so that snapshot tests are stable across runs.
pub trait Reporter {
    /// Renders the report.
    ///
    /// # Errors
    ///
    /// Returns an error string if rendering fails (for example if a required
    /// file cannot be read for source snippets).  In practice `pretty` never
    /// fails; JSON and SARIF can if serialisation fails.
    fn render(&self, report: &Report) -> Result<String, String>;
}
