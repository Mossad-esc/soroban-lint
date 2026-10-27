//! Per-crate call graph built from the parsed AST.
//!
//! Nodes represent functions defined in the file.  Edges represent calls
//! resolved within the same file.  Call-sites (including unresolved SDK
//! method calls) are stored on each node so that predicates like
//! "reaches require_auth" match even though `require_auth` is never declared
//! in the file being analysed.
//!
//! # Cycle safety
//!
//! The DFS uses a visited-set; recursion never loops.

use std::collections::{HashMap, HashSet};

use petgraph::graph::{DiGraph, NodeIndex};
use syn::visit::Visit;
use syn::{Block, Expr, ExprCall, ExprMethodCall, File, ImplItem, Item};

// ─── Public types ─────────────────────────────────────────────────────────────

/// A directed call graph for one file.
pub struct CallGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
    /// All call-site names recorded for each node (resolved + unresolved).
    callsites: HashMap<NodeIndex, Vec<String>>,
}

impl CallGraph {
    /// Builds a call graph from a parsed file.
    #[must_use]
    pub fn build(file: &File) -> Self {
        let mut cg = Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            callsites: HashMap::new(),
        };
        cg.collect_nodes(file);
        cg.collect_edges(file);
        cg
    }

    /// Returns `true` when `start`, or any function it transitively calls,
    /// has a call-site whose name satisfies `predicate`.
    ///
    /// This matches both intra-crate calls (expanded recursively) and
    /// external calls (SDK methods, etc.).
    #[must_use]
    pub fn reaches<F>(&self, start: &str, predicate: &F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        let Some(&start_idx) = self.node_map.get(start) else {
            return false;
        };
        let mut visited = HashSet::new();
        self.dfs(start_idx, predicate, &mut visited)
    }

    fn get_or_insert(&mut self, name: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(name) {
            return idx;
        }
        let idx = self.graph.add_node(name.to_owned());
        self.node_map.insert(name.to_owned(), idx);
        idx
    }

    fn collect_nodes(&mut self, file: &File) {
        for item in &file.items {
            match item {
                Item::Fn(f) => {
                    self.get_or_insert(&f.sig.ident.to_string());
                }
                Item::Impl(imp) => {
                    for impl_item in &imp.items {
                        if let ImplItem::Fn(m) = impl_item {
                            self.get_or_insert(&m.sig.ident.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_edges(&mut self, file: &File) {
        for item in &file.items {
            match item {
                Item::Fn(f) => {
                    let caller_name = f.sig.ident.to_string();
                    let caller_idx = self.get_or_insert(&caller_name);
                    let sites = all_callsites_in_block(&f.block);
                    self.callsites
                        .entry(caller_idx)
                        .or_default()
                        .extend(sites.iter().cloned());
                    for callee in &sites {
                        if self.node_map.contains_key(callee.as_str()) {
                            let callee_idx = self.get_or_insert(callee);
                            self.graph.add_edge(caller_idx, callee_idx, ());
                        }
                    }
                }
                Item::Impl(imp) => {
                    for impl_item in &imp.items {
                        if let ImplItem::Fn(m) = impl_item {
                            let caller_name = m.sig.ident.to_string();
                            let caller_idx = self.get_or_insert(&caller_name);
                            let sites = all_callsites_in_block(&m.block);
                            self.callsites
                                .entry(caller_idx)
                                .or_default()
                                .extend(sites.iter().cloned());
                            for callee in &sites {
                                if self.node_map.contains_key(callee.as_str()) {
                                    let callee_idx = self.get_or_insert(callee);
                                    self.graph.add_edge(caller_idx, callee_idx, ());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn dfs<F>(&self, node: NodeIndex, predicate: &F, visited: &mut HashSet<NodeIndex>) -> bool
    where
        F: Fn(&str) -> bool,
    {
        if !visited.insert(node) {
            return false;
        }
        // Check own name.
        if predicate(self.graph[node].as_str()) {
            return true;
        }
        // Check all call-sites on this node (catches unresolved SDK calls).
        if let Some(sites) = self.callsites.get(&node) {
            for site in sites {
                if predicate(site.as_str()) {
                    return true;
                }
            }
        }
        // Recurse into resolved callees.
        for neighbor in self.graph.neighbors(node) {
            if self.dfs(neighbor, predicate, visited) {
                return true;
            }
        }
        false
    }
}

// ─── Call-site collector ──────────────────────────────────────────────────────

fn all_callsites_in_block(block: &Block) -> Vec<String> {
    let mut v = CalleeVisitor {
        callees: Vec::new(),
    };
    v.visit_block(block);
    v.callees
}

struct CalleeVisitor {
    callees: Vec<String>,
}

impl<'ast> Visit<'ast> for CalleeVisitor {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(ep) = call.func.as_ref() {
            if let Some(last) = ep.path.segments.last() {
                self.callees.push(last.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.callees.push(call.method.to_string());
        syn::visit::visit_expr_method_call(self, call);
    }
}

// ─── Rule helpers ─────────────────────────────────────────────────────────────

/// `true` when `fn_name` or any transitive callee invokes `require_auth` or
/// `require_auth_for_args`.
#[must_use]
pub fn fn_has_auth(cg: &CallGraph, fn_name: &str) -> bool {
    cg.reaches(fn_name, &|n| {
        n == "require_auth" || n == "require_auth_for_args"
    })
}

/// `true` when `fn_name` or any transitive callee invokes `extend_ttl`.
#[must_use]
pub fn fn_has_extend_ttl(cg: &CallGraph, fn_name: &str) -> bool {
    cg.reaches(fn_name, &|n| n == "extend_ttl")
}

/// `true` when `fn_name` or any transitive callee invokes `set` or `remove`.
#[must_use]
pub fn fn_writes_storage(cg: &CallGraph, fn_name: &str) -> bool {
    cg.reaches(fn_name, &|n| n == "set" || n == "remove")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn build(file: syn::File) -> CallGraph {
        CallGraph::build(&file)
    }

    #[test]
    fn direct_call_is_reachable() {
        let file: syn::File = parse_quote! {
            fn caller() { callee(); }
            fn callee() {}
        };
        let cg = build(file);
        assert!(cg.reaches("caller", &|n| n == "callee"));
        assert!(!cg.reaches("callee", &|n| n == "caller"));
    }

    #[test]
    fn transitive_call_is_reachable() {
        let file: syn::File = parse_quote! {
            fn a() { b(); }
            fn b() { c(); }
            fn c() {}
        };
        let cg = build(file);
        assert!(cg.reaches("a", &|n| n == "c"));
        assert!(!cg.reaches("c", &|n| n == "a"));
    }

    #[test]
    fn method_call_is_reachable() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn transfer(&self) { self.check_auth(); }
                fn check_auth(&self) {}
            }
        };
        let cg = build(file);
        assert!(cg.reaches("transfer", &|n| n == "check_auth"));
    }

    #[test]
    fn cycle_does_not_loop_forever() {
        let file: syn::File = parse_quote! {
            fn a() { b(); }
            fn b() { a(); }
        };
        let cg = build(file);
        assert!(!cg.reaches("a", &|_| false));
    }

    #[test]
    fn self_recursion_is_safe() {
        let file: syn::File = parse_quote! {
            fn fib(n: u64) -> u64 { fib(n - 1u64) }
        };
        let cg = build(file);
        assert!(!cg.reaches("fib", &|_| false));
    }

    #[test]
    fn auth_in_helper_credited_to_entrypoint() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn transfer(env: Env, from: Address) {
                    Self::do_auth(&from);
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
                fn do_auth(who: &Address) { who.require_auth(); }
            }
        };
        let cg = build(file);
        assert!(
            fn_has_auth(&cg, "transfer"),
            "auth in helper must be credited"
        );
    }

    #[test]
    fn shared_helper_credits_both_callers() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn mint(env: Env) {
                    Self::assert_admin(&env);
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
                pub fn burn(env: Env) {
                    Self::assert_admin(&env);
                    env.storage().persistent().remove(&DataKey::X);
                }
                fn assert_admin(env: &Env) { env.require_auth(); }
            }
        };
        let cg = build(file);
        assert!(fn_has_auth(&cg, "mint"));
        assert!(fn_has_auth(&cg, "burn"));
    }

    #[test]
    fn ttl_extend_in_helper_detected() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn register(env: Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                    Self::bump_ttl(&env);
                }
                fn bump_ttl(env: &Env) {
                    env.storage().persistent().extend_ttl(&DataKey::X, 100u32, 1000u32);
                }
            }
        };
        let cg = build(file);
        assert!(fn_has_extend_ttl(&cg, "register"));
    }

    #[test]
    fn storage_write_in_helper_detected() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn entry(env: Env) { Self::write_data(&env); }
                fn write_data(env: &Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
            }
        };
        let cg = build(file);
        assert!(fn_writes_storage(&cg, "entry"));
    }

    #[test]
    fn unresolved_call_does_not_panic() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn f(env: Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
            }
        };
        let cg = build(file);
        assert!(!cg.reaches("f", &|n| n == "nonexistent"));
    }
}
