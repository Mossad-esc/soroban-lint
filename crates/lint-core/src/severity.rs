//! Finding severity levels.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The severity of a finding.
///
/// The ordering is `Note < Warning < Error`, so severities can be compared
/// directly when applying `--min-severity` and `--fail-on` thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Style or best-practice suggestion.
    Note,
    /// Risky pattern that deserves review.
    Warning,
    /// Likely a real bug or vulnerability.
    Error,
}

impl Severity {
    /// Returns the lowercase wire representation (`note`, `warning`, `error`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

/// Error returned when a string cannot be parsed into a [`Severity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSeverityError {
    input: String,
}

impl fmt::Display for ParseSeverityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid severity `{}`: expected one of `note`, `warning`, `error`",
            self.input
        )
    }
}

impl std::error::Error for ParseSeverityError {}

impl FromStr for Severity {
    type Err = ParseSeverityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "note" => Ok(Self::Note),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            other => Err(ParseSeverityError {
                input: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_note_warning_error() {
        assert!(Severity::Note < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Note < Severity::Error);
    }

    #[test]
    fn sorts_ascending() {
        let mut levels = vec![Severity::Error, Severity::Note, Severity::Warning];
        levels.sort();
        assert_eq!(
            levels,
            vec![Severity::Note, Severity::Warning, Severity::Error]
        );
    }

    #[test]
    fn parses_known_values() {
        assert_eq!("note".parse::<Severity>(), Ok(Severity::Note));
        assert_eq!("warning".parse::<Severity>(), Ok(Severity::Warning));
        assert_eq!("error".parse::<Severity>(), Ok(Severity::Error));
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!("  warning ".parse::<Severity>(), Ok(Severity::Warning));
    }

    #[test]
    fn rejects_unknown_values() {
        for input in ["", "warn", "ERROR", "fatal"] {
            assert!(
                input.parse::<Severity>().is_err(),
                "expected `{input}` to fail"
            );
        }
    }

    #[test]
    fn display_round_trips() {
        for level in [Severity::Note, Severity::Warning, Severity::Error] {
            assert_eq!(level.to_string().parse::<Severity>(), Ok(level));
        }
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Severity::Error).unwrap(),
            "\"error\""
        );
    }
}
