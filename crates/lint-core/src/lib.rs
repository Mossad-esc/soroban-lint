//! Core types, rule engine, and analysis context for `soroban-lint`.

pub mod finding;
pub mod model;
pub mod rule;
pub mod severity;

pub use finding::Finding;
pub use model::ContractModel;
pub use rule::{Rule, RuleConfig, RuleContext};
pub use severity::{ParseSeverityError, Severity};
