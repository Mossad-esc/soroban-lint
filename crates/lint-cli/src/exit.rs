//! The process exit-code contract.
//!
//! Mirrors the README's exit-code table: `0` clean, `1` findings at or above
//! the `--fail-on` level, and `2` for usage, configuration, or internal errors.

use lint_core::{Finding, Severity};

/// Process exit codes used by `soroban-lint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// No findings at or above the `--fail-on` level.
    Clean = 0,
    /// At least one finding at or above the `--fail-on` level.
    Findings = 1,
    /// Usage, configuration, or internal error.
    Error = 2,
}

impl ExitCode {
    /// The numeric value passed to the operating system.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }
}

/// Computes the exit code for a completed run from its findings.
///
/// Only `Clean` and `Findings` can result from findings; the `Error` outcome is
/// reserved for failures that happen before or around analysis.
#[must_use]
pub fn code_for_findings(findings: &[Finding], fail_on: Severity) -> ExitCode {
    if findings.iter().any(|finding| finding.severity >= fail_on) {
        ExitCode::Findings
    } else {
        ExitCode::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::Finding;

    fn finding(severity: Severity) -> Finding {
        Finding::new(
            "SOR001",
            "missing-require-auth",
            severity,
            "x",
            "src/lib.rs",
            1,
            1,
        )
    }

    #[test]
    fn clean_when_no_findings() {
        assert_eq!(code_for_findings(&[], Severity::Error), ExitCode::Clean);
        assert_eq!(code_for_findings(&[], Severity::Error).code(), 0);
    }

    #[test]
    fn findings_when_at_or_above_threshold() {
        let findings = vec![finding(Severity::Error)];
        assert_eq!(
            code_for_findings(&findings, Severity::Error),
            ExitCode::Findings
        );
        assert_eq!(code_for_findings(&findings, Severity::Error).code(), 1);
    }

    #[test]
    fn below_threshold_is_clean() {
        let findings = vec![finding(Severity::Warning)];
        assert_eq!(
            code_for_findings(&findings, Severity::Error),
            ExitCode::Clean
        );
    }

    #[test]
    fn error_outcome_uses_code_two() {
        assert_eq!(ExitCode::Error.code(), 2);
    }

    #[test]
    fn all_outcomes_have_documented_codes() {
        assert_eq!(ExitCode::Clean.code(), 0);
        assert_eq!(ExitCode::Findings.code(), 1);
        assert_eq!(ExitCode::Error.code(), 2);
    }
}
