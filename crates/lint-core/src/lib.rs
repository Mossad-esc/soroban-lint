//! Core types, rule engine, and analysis context for `soroban-lint`.

pub mod discovery;
pub mod finding;
pub mod model;
pub mod registry;
pub mod rule;
pub mod severity;

pub use discovery::{discover, Discovery, DiscoveryOptions, ParseError, SourceFile};
pub use finding::Finding;
pub use model::ContractModel;
pub use registry::{RegisteredRule, RuleRegistry, RuleStatus};
pub use rule::{Rule, RuleConfig, RuleContext};
pub use severity::{ParseSeverityError, Severity};
