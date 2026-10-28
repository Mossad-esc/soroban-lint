//! Core types, rule engine, and analysis context for `soroban-lint`.

pub mod baseline;
pub mod callgraph;
pub mod config;
pub mod discovery;
pub mod finding;
pub mod model;
pub mod registry;
pub mod rule;
pub mod severity;
pub mod suppression;
pub mod util;

pub use baseline::{
    apply_baseline, baseline_from_findings, fingerprint as baseline_fingerprint, load_baseline,
    save_baseline, Baseline, BaselineError,
};
pub use callgraph::{fn_has_auth, fn_has_extend_ttl, fn_writes_storage, CallGraph};
pub use config::{
    find_config, load_config, ConfigError, GeneralConfig, LintConfig, ResolvedRuleConfig,
    DEFAULT_CONFIG_CONTENT,
};
pub use discovery::{discover, Discovery, DiscoveryOptions, ParseError, SourceFile};
pub use finding::Finding;
pub use model::{
    ContractError, ContractImpl, ContractModel, ContractStruct, ContractType, ContractTypeKind,
    Entrypoint, EnumVariant, Function, HelperFn,
};
pub use registry::{RegisteredRule, RuleRegistry, RuleStatus};
pub use rule::{Rule, RuleConfig, RuleContext};
pub use severity::{ParseSeverityError, Severity};
pub use suppression::{apply as apply_suppressions, scan as scan_suppressions, SuppressionMap};
