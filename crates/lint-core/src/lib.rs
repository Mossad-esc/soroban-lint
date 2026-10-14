//! Core types, rule engine, and analysis context for `soroban-lint`.

pub mod discovery;
pub mod finding;
pub mod model;
pub mod registry;
pub mod rule;
pub mod severity;
pub mod util;

pub use discovery::{discover, Discovery, DiscoveryOptions, ParseError, SourceFile};
pub use finding::Finding;
pub use model::{
    ContractError, ContractImpl, ContractModel, ContractStruct, ContractType, ContractTypeKind,
    Entrypoint, EnumVariant, Function, HelperFn,
};
pub use registry::{RegisteredRule, RuleRegistry, RuleStatus};
pub use rule::{Rule, RuleConfig, RuleContext};
pub use severity::{ParseSeverityError, Severity};
