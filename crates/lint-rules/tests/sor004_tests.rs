//! Snapshot tests for SOR004 `critical-data-in-temporary`.

mod harness;

use lint_core::RuleConfig;
use lint_core::{ContractModel, RuleContext};
use lint_rules::sor004::Sor004;

/// The README's primary example: Admin written to temporary storage.
#[test]
fn sor004_vulnerable_admin_in_temporary() {
    assert_rule_snapshot!(
        Sor004,
        "SOR004",
        "vulnerable",
        "sor004_admin_in_temporary.rs"
    );
}

/// Enum variant with constructor arguments (DataKey::Balance(addr)).
#[test]
fn sor004_vulnerable_balance_variant() {
    assert_rule_snapshot!(Sor004, "SOR004", "vulnerable", "sor004_balance_variant.rs");
}

/// Non-critical key (nonce cache) in temporary storage: must not trigger.
#[test]
fn sor004_safe_nonce_in_temporary() {
    assert_rule_snapshot!(Sor004, "SOR004", "safe", "sor004_nonce_in_temporary.rs");
}

/// Admin correctly stored in instance storage: must not trigger.
#[test]
fn sor004_safe_admin_in_instance() {
    assert_rule_snapshot!(Sor004, "SOR004", "safe", "sor004_admin_in_instance.rs");
}

/// Custom critical_keys config: only "secret" is critical; "admin" is not.
#[test]
fn sor004_custom_critical_keys() {
    use lint_core::Rule;
    use lint_rules::sor004::Sor004;

    let source = r#"
        #[contractimpl]
        impl Contract {
            pub fn store(env: Env, val: u32) {
                env.storage().temporary().set(&DataKey::Secret, &val);
                env.storage().temporary().set(&DataKey::Admin, &val);
            }
        }
    "#;

    let ast = syn::parse_str::<syn::File>(source).expect("parse");
    let model = ContractModel::build(&ast);
    let config = RuleConfig {
        critical_keys: Some(vec!["secret".to_owned()]),
        ..RuleConfig::default()
    };
    let path = std::path::Path::new("test.rs");
    let ctx = RuleContext::new(&ast, path, source, &model, &config);
    let findings = Sor004.check(&ctx);

    // Only "Secret" should be flagged; "Admin" is not in the custom list.
    assert_eq!(findings.len(), 1, "expected one finding for 'secret'");
    assert!(
        findings[0].message.contains("Secret"),
        "finding should mention Secret"
    );
}
