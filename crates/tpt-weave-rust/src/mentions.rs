//! Body/signature mention extraction for reference and caller/callee
//! relationships (todo.md Phase 3: "Build reference relationships",
//! "Track callers/callees where possible").
//!
//! A *mention* is one identifier occurrence attributable to a symbol: path
//! segments in signatures, generic bounds, expression paths (`foo(..)`),
//! and method calls (`x.bar()`). Resolution against the symbol table — with
//! module/package tiering — happens in `tpt-weave-graph`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use syn::visit::Visit;
use syn::{Block, Expr, ExprCall, ExprMethodCall, Item, Signature, Type};

/// Syntactic form of a [`Mention`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionKind {
    /// General identifier occurrence (paths, types, bounds).
    Path,
    /// `foo(..)` — a direct call expression.
    Call,
    /// `x.bar(..)` — a method call.
    MethodCall,
}

impl MentionKind {
    /// Dedupe precedence: an occurrence seen as a call wins over a plain
    /// path occurrence of the same name.
    fn rank(self) -> u8 {
        match self {
            MentionKind::Path => 1,
            MentionKind::MethodCall => 2,
            MentionKind::Call => 3,
        }
    }
}

/// One identifier occurrence attributable to a symbol.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mention {
    /// Canonical key ([`tpt_weave_core::SymbolId::canonical_key`]) of the
    /// symbol containing the occurrence.
    pub from: String,
    /// Referenced identifier (a path segment or method name).
    pub name: String,
    pub kind: MentionKind,
}

/// Collects mentions for one symbol; deduplicated per name with the most
/// specific [`MentionKind`] kept.
pub(crate) fn walk_item(from: &str, item: &Item) -> Vec<Mention> {
    let mut walker = Walker::new(from);
    match item {
        Item::Fn(node) => walker.visit_item_fn(node),
        Item::Struct(node) => walker.visit_item_struct(node),
        Item::Enum(node) => walker.visit_item_enum(node),
        Item::Trait(node) => walker.visit_item_trait(node),
        Item::Impl(node) => walker.visit_item_impl(node),
        Item::Const(node) => walker.visit_item_const(node),
        Item::Static(node) => walker.visit_item_static(node),
        Item::Type(node) => walker.visit_item_type(node),
        // Modules and macros are containers/definitions without mentionable
        // types of their own; nested definitions get their own pass.
        _ => {}
    }
    walker.finish()
}

/// Mentions of one function-like symbol (free fns, trait methods, impl
/// methods) from its signature and optional body.
pub(crate) fn walk_fn(from: &str, sig: &Signature, body: Option<&Block>) -> Vec<Mention> {
    let mut walker = Walker::new(from);
    walker.visit_signature(sig);
    if let Some(body) = body {
        walker.visit_block(body);
    }
    walker.finish()
}

/// Mentions from a type and optional value (constants, associated
/// constants).
pub(crate) fn walk_type_expr(from: &str, ty: &Type, expr: Option<&Expr>) -> Vec<Mention> {
    let mut walker = Walker::new(from);
    walker.visit_type(ty);
    if let Some(expr) = expr {
        walker.visit_expr(expr);
    }
    walker.finish()
}

struct Walker {
    from: String,
    /// Simple name of the symbol being walked — its own identifier appears
    /// in signatures and is not a reference to anything else.
    own: String,
    seen: BTreeMap<String, MentionKind>,
}

impl Walker {
    fn new(from: &str) -> Self {
        let base = from.split('#').next().unwrap_or(from);
        Self {
            from: from.to_string(),
            own: base.rsplit("::").next().unwrap_or_default().to_string(),
            seen: BTreeMap::new(),
        }
    }

    fn record(&mut self, name: String, kind: MentionKind) {
        if name.is_empty() || name == self.own {
            return;
        }
        let entry = self.seen.entry(name).or_insert(kind);
        if kind.rank() > entry.rank() {
            *entry = kind;
        }
    }

    fn finish(self) -> Vec<Mention> {
        self.seen
            .into_iter()
            .map(|(name, kind)| Mention {
                from: self.from.clone(),
                name,
                kind,
            })
            .collect()
    }
}

impl<'ast> Visit<'ast> for Walker {
    // Nested definitions are recorded by the extractor's own walk with
    // their own symbol identity — never absorbed into the parent.
    fn visit_item(&mut self, _node: &'ast Item) {}
    fn visit_trait_item(&mut self, _node: &'ast syn::TraitItem) {}
    fn visit_impl_item(&mut self, _node: &'ast syn::ImplItem) {}

    fn visit_path(&mut self, node: &'ast syn::Path) {
        for segment in &node.segments {
            self.record(segment.ident.to_string(), MentionKind::Path);
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            if let Some(last) = path.path.segments.last() {
                self.record(last.ident.to_string(), MentionKind::Call);
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.record(node.method.to_string(), MentionKind::MethodCall);
        syn::visit::visit_expr_method_call(self, node);
    }
}
