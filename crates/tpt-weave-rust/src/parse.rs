//! syn-based extraction of Rust items (spec.md section 8.2, todo.md Phase 2
//! "Source parsing").
//!
//! Each file is parsed independently with [`syn::parse_file`]; line/column
//! information comes from `proc-macro2`'s `span-locations` feature and
//! points at the item's name (or the `impl` keyword). Signatures are
//! rendered from token streams and normalised to a single line.

use crate::mentions::{self, Mention};
use crate::record::{SymbolRecord, Visibility};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use serde::{Deserialize, Serialize};
use std::fmt;
use syn::spanned::Spanned;
use syn::{
    Attribute, Item, Type, Visibility as SynVisibility,
};

/// Failure to parse a Rust source file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    /// 1-based line of the offending token.
    pub line: u32,
    /// 1-based column of the offending token.
    pub column: u32,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Everything needed to derive stable [`SymbolId`](tpt_weave_core::SymbolId)s
/// for one file.
#[derive(Clone, Debug)]
pub struct FileInput<'a> {
    /// Repository the file belongs to.
    pub repository: &'a tpt_weave_core::RepositoryId,
    /// Cargo package (crate) name.
    pub package: &'a str,
    /// Repository-relative path of the file being parsed.
    pub path: &'a str,
    /// Module path prefix for the file (e.g. from an out-of-line `mod`
    /// chain); empty for a crate-root file.
    pub module_prefix: &'a [&'a str],
}

/// Symbols and mentions extracted from one file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedFile {
    /// The path as passed in.
    pub path: String,
    /// Symbols in depth-first item order.
    pub symbols: Vec<SymbolRecord>,
    /// Identifier occurrences attributable to those symbols.
    pub mentions: Vec<Mention>,
}

/// Parses `source` and extracts all top-level and nested items.
pub fn parse_file(input: &FileInput<'_>, source: &str) -> Result<ParsedFile, ParseError> {
    let file = syn::parse_file(source).map_err(|err| {
        let start = err.span().start();
        ParseError {
            message: err.to_string(),
            line: start.line as u32,
            column: start.column as u32 + 1,
        }
    })?;

    let mut extractor = Extractor {
        input,
        module_path: input.module_prefix.iter().map(|s| (*s).to_string()).collect(),
        symbols: Vec::new(),
        mentions: Vec::new(),
    };
    extractor.walk_items(&file.items, None);
    Ok(ParsedFile {
        path: input.path.to_string(),
        symbols: extractor.symbols,
        mentions: extractor.mentions,
    })
}

struct Extractor<'a> {
    input: &'a FileInput<'a>,
    module_path: Vec<String>,
    symbols: Vec<SymbolRecord>,
    mentions: Vec<Mention>,
}

impl<'a> Extractor<'a> {
    /// Pushes one record and returns its id so children can reference it.
    #[allow(clippy::too_many_arguments)]
    fn push_symbol(
        &mut self,
        span: Span,
        visibility: &SynVisibility,
        kind: tpt_weave_core::SymbolKind,
        name: String,
        signature: String,
        attributes: &[Attribute],
        parent: Option<&tpt_weave_core::SymbolId>,
    ) -> tpt_weave_core::SymbolId {
        let id = tpt_weave_core::SymbolId {
            repository: self.input.repository.clone(),
            package: self.input.package.to_string(),
            module: self.module_path.join("::"),
            name,
            kind,
        };
        let start = span.start();
        self.symbols.push(SymbolRecord {
            id: id.clone(),
            file: self.input.path.to_string(),
            line: start.line as u32,
            column: start.column as u32 + 1,
            visibility: visibility_of(visibility),
            signature,
            attributes: attributes
                .iter()
                .map(|attr| normalize(&attr.to_token_stream().to_string()))
                .collect(),
            parent: parent.map(|p| p.canonical_key()),
        });
        id
    }

    fn walk_items(&mut self, items: &[Item], parent: Option<&tpt_weave_core::SymbolId>) {
        for item in items {
            self.walk_item(item, parent);
        }
    }

    fn walk_item(&mut self, item: &Item, parent: Option<&tpt_weave_core::SymbolId>) {
        match item {
            Item::Mod(module) => {
                let name = module.ident.to_string();
                let vis = &module.vis;
                let mod_token = &module.mod_token;
                let ident = &module.ident;
                let signature = normalize(&quote!(#vis #mod_token #ident).to_string());
                let id = self.push_symbol(
                    module.ident.span(),
                    &module.vis,
                    tpt_weave_core::SymbolKind::Module,
                    name.clone(),
                    signature,
                    &module.attrs,
                    parent,
                );
                if let Some((_, items)) = &module.content {
                    self.module_path.push(name);
                    self.walk_items(items, Some(&id));
                    self.module_path.pop();
                }
            }
            Item::Struct(st) => {
                let vis = &st.vis;
                let struct_token = &st.struct_token;
                let ident = &st.ident;
                let generics = &st.generics;
                let where_clause = &st.generics.where_clause;
                let signature =
                    normalize(&quote!(#vis #struct_token #ident #generics #where_clause).to_string());
                let id = self.push_symbol(
                    st.ident.span(),
                    &st.vis,
                    tpt_weave_core::SymbolKind::Struct,
                    st.ident.to_string(),
                    signature,
                    &st.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Enum(en) => {
                let vis = &en.vis;
                let enum_token = &en.enum_token;
                let ident = &en.ident;
                let generics = &en.generics;
                let where_clause = &en.generics.where_clause;
                let signature =
                    normalize(&quote!(#vis #enum_token #ident #generics #where_clause).to_string());
                let id = self.push_symbol(
                    en.ident.span(),
                    &en.vis,
                    tpt_weave_core::SymbolKind::Enum,
                    en.ident.to_string(),
                    signature,
                    &en.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Fn(fun) => {
                let vis = &fun.vis;
                let sig = &fun.sig;
                let signature = normalize(&quote!(#vis #sig).to_string());
                let id = self.push_symbol(
                    fun.sig.ident.span(),
                    &fun.vis,
                    tpt_weave_core::SymbolKind::Function,
                    fun.sig.ident.to_string(),
                    signature,
                    &fun.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Const(konst) => {
                let vis = &konst.vis;
                let const_token = &konst.const_token;
                let ident = &konst.ident;
                let colon_token = &konst.colon_token;
                let ty = &konst.ty;
                let eq_token = &konst.eq_token;
                let expr = &konst.expr;
                let semi_token = &konst.semi_token;
                let signature = normalize(
                    &quote!(#vis #const_token #ident #colon_token #ty #eq_token #expr #semi_token)
                        .to_string(),
                );
                let id = self.push_symbol(
                    konst.ident.span(),
                    &konst.vis,
                    tpt_weave_core::SymbolKind::Constant,
                    konst.ident.to_string(),
                    signature,
                    &konst.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Static(stat) => {
                // Statics are recorded as constants (SymbolKind has no
                // separate static variant); the signature keeps `static`.
                let vis = &stat.vis;
                let static_token = &stat.static_token;
                let mutability = &stat.mutability;
                let ident = &stat.ident;
                let colon_token = &stat.colon_token;
                let ty = &stat.ty;
                let eq_token = &stat.eq_token;
                let expr = &stat.expr;
                let semi_token = &stat.semi_token;
                let signature = normalize(
                    &quote!(#vis #static_token #mutability #ident #colon_token #ty #eq_token #expr #semi_token)
                        .to_string(),
                );
                let id = self.push_symbol(
                    stat.ident.span(),
                    &stat.vis,
                    tpt_weave_core::SymbolKind::Constant,
                    stat.ident.to_string(),
                    signature,
                    &stat.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Type(alias) => {
                let vis = &alias.vis;
                let type_token = &alias.type_token;
                let ident = &alias.ident;
                let generics = &alias.generics;
                let eq_token = &alias.eq_token;
                let ty = &alias.ty;
                let semi_token = &alias.semi_token;
                let signature = normalize(
                    &quote!(#vis #type_token #ident #generics #eq_token #ty #semi_token)
                        .to_string(),
                );
                let id = self.push_symbol(
                    alias.ident.span(),
                    &alias.vis,
                    tpt_weave_core::SymbolKind::TypeAlias,
                    alias.ident.to_string(),
                    signature,
                    &alias.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
            }
            Item::Macro(mac) => {
                // `macro_rules!` definitions only (macro invocations are not
                // symbol definitions).
                if let (Some(ident), true) = (&mac.ident, mac.mac.path.is_ident("macro_rules")) {
                    let path = &mac.mac.path;
                    let bang = &mac.mac.bang_token;
                    let signature = normalize(&quote!(#path #bang #ident).to_string());
                    self.push_symbol(
                        ident.span(),
                        &SynVisibility::Inherited,
                        tpt_weave_core::SymbolKind::Macro,
                        ident.to_string(),
                        signature,
                        &mac.attrs,
                        parent,
                    );
                }
            }
            Item::Trait(tr) => {
                let vis = &tr.vis;
                let unsafety = &tr.unsafety;
                let trait_token = &tr.trait_token;
                let ident = &tr.ident;
                let generics = &tr.generics;
                let mut signature_tokens =
                    quote!(#vis #unsafety #trait_token #ident #generics);
                if let Some(colon) = &tr.colon_token {
                    colon.to_tokens(&mut signature_tokens);
                    for pair in tr.supertraits.pairs() {
                        match pair {
                            syn::punctuated::Pair::Punctuated(bound, punct) => {
                                bound.to_tokens(&mut signature_tokens);
                                punct.to_tokens(&mut signature_tokens);
                            }
                            syn::punctuated::Pair::End(bound) => {
                                bound.to_tokens(&mut signature_tokens);
                            }
                        }
                    }
                }
                if let Some(where_clause) = &tr.generics.where_clause {
                    where_clause.to_tokens(&mut signature_tokens);
                }
                let signature = normalize(&signature_tokens.to_string());
                let id = self.push_symbol(
                    tr.ident.span(),
                    &tr.vis,
                    tpt_weave_core::SymbolKind::Trait,
                    tr.ident.to_string(),
                    signature,
                    &tr.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
                // Trait methods (declarations) become `Trait::method`.
                for trait_item in &tr.items {
                    if let syn::TraitItem::Fn(method) = trait_item {
                        let sig = &method.sig;
                        let method_signature = normalize(&quote!(#sig).to_string());
                        let method_id = self.push_symbol(
                            method.sig.ident.span(),
                            &SynVisibility::Inherited,
                            tpt_weave_core::SymbolKind::Method,
                            format!("{}::{}", tr.ident, method.sig.ident),
                            method_signature,
                            &method.attrs,
                            Some(&id),
                        );
                        self.mentions.extend(mentions::walk_fn(
                            &method_id.canonical_key(),
                            sig,
                            method.default.as_ref(),
                        ));
                    }
                }
            }
            Item::Impl(im) => {
                let defaultness = &im.defaultness;
                let unsafety = &im.unsafety;
                let impl_token = &im.impl_token;
                let generics = &im.generics;
                let self_ty = &im.self_ty;
                let type_name = type_name(self_ty);
                let mut signature_tokens =
                    quote!(#defaultness #unsafety #impl_token #generics);
                let impl_name = if let Some((bang, trait_path, for_token)) = &im.trait_ {
                    if let Some(bang) = bang {
                        bang.to_tokens(&mut signature_tokens);
                    }
                    trait_path.to_tokens(&mut signature_tokens);
                    for_token.to_tokens(&mut signature_tokens);
                    format!(
                        "<{} as {}>",
                        type_name,
                        normalize(&trait_path.to_token_stream().to_string())
                    )
                } else {
                    type_name.clone()
                };
                self_ty.to_tokens(&mut signature_tokens);
                if let Some(where_clause) = &im.generics.where_clause {
                    where_clause.to_tokens(&mut signature_tokens);
                }
                let signature = normalize(&signature_tokens.to_string());
                let id = self.push_symbol(
                    im.impl_token.span(),
                    &SynVisibility::Inherited,
                    tpt_weave_core::SymbolKind::Impl,
                    impl_name,
                    signature,
                    &im.attrs,
                    parent,
                );
                self.mentions.extend(mentions::walk_item(&id.canonical_key(), item));
                // Members become `Type::member`; the enclosing impl's
                // canonical key in `parent` disambiguates same-named methods
                // from different trait impls.
                for impl_item in &im.items {
                    match impl_item {
                        syn::ImplItem::Fn(method) => {
                            let vis = &method.vis;
                            let sig = &method.sig;
                            let method_signature = normalize(&quote!(#vis #sig).to_string());
                            let method_id = self.push_symbol(
                                method.sig.ident.span(),
                                &method.vis,
                                tpt_weave_core::SymbolKind::Method,
                                format!("{}::{}", type_name, method.sig.ident),
                                method_signature,
                                &method.attrs,
                                Some(&id),
                            );
                            self.mentions.extend(mentions::walk_fn(
                                &method_id.canonical_key(),
                                sig,
                                Some(&method.block),
                            ));
                        }
                        syn::ImplItem::Const(konst) => {
                            let vis = &konst.vis;
                            let defaultness = &konst.defaultness;
                            let const_token = &konst.const_token;
                            let ident = &konst.ident;
                            let colon_token = &konst.colon_token;
                            let ty = &konst.ty;
                            let eq_token = &konst.eq_token;
                            let expr = &konst.expr;
                            let semi_token = &konst.semi_token;
                            let const_signature = normalize(
                                &quote!(#vis #defaultness #const_token #ident #colon_token #ty #eq_token #expr #semi_token)
                                    .to_string(),
                            );
                            let const_id = self.push_symbol(
                                konst.ident.span(),
                                &konst.vis,
                                tpt_weave_core::SymbolKind::Constant,
                                format!("{}::{}", type_name, konst.ident),
                                const_signature,
                                &konst.attrs,
                                Some(&id),
                            );
                            self.mentions.extend(mentions::walk_type_expr(
                                &const_id.canonical_key(),
                                &konst.ty,
                                Some(&konst.expr),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            // `use`, `extern crate`, foreign modules and verbatim items are
            // not symbol definitions and are skipped.
            _ => {}
        }
    }




}

/// Renders a token stream as one normalised line (deterministic spacing:
/// `fn resize(image: &PixelBuffer) -> Result<Image>`).
pub(crate) fn normalize(raw: &str) -> String {
    let mut out = raw.to_string();
    for (from, to) in [
        (" (", "("),
        ("( ", "("),
        (" )", ")"),
        (" :", ":"),
        (" ,", ","),
        (" ;", ";"),
        (" <", "<"),
        ("< ", "<"),
        (" >", ">"),
        (" [", "["),
        ("[ ", "["),
        (" ]", "]"),
        (" !", "!"),
        (" ::", "::"),
        (":: ", "::"),
        ("& ", "&"),
        ("# [", "#["),
    ] {
        out = out.replace(from, to);
    }
    out
}

/// Converts a `syn` visibility into the extracted model's [`Visibility`].
pub(crate) fn visibility_of(vis: &SynVisibility) -> Visibility {
    match vis {
        SynVisibility::Public(_) => Visibility::Public,
        SynVisibility::Inherited => Visibility::Inherited,
        SynVisibility::Restricted(restricted) => {
            let path = normalize(&restricted.path.to_token_stream().to_string());
            if restricted.in_token.is_none() {
                if path == "crate" {
                    return Visibility::Crate;
                }
                if path == "self" {
                    return Visibility::Inherited;
                }
            }
            Visibility::Restricted(path)
        }
    }
}

/// Best-effort simple name of a type for impl/method naming
/// (`PixelBuffer<u8>` -> `PixelBuffer`); falls back to the rendered type.
pub(crate) fn type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(last) = type_path.path.segments.last() {
            return last.ident.to_string();
        }
    }
    normalize(&ty.to_token_stream().to_string())
}
