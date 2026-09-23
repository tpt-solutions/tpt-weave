//! Source skeleton generation (todo.md Phase 4, spec.md section 9 levels 3-4).
//!
//! Renders a source file with function bodies removed while preserving
//! signatures, type definitions, impl relationships, attributes, module
//! structure and `// path:line` source references. Bodies of *selected*
//! symbols can be kept ([`skeleton_with`]) to produce level-4 targeted
//! implementation views.

use crate::parse::{FileInput, ParseError, type_name};
use proc_macro2::Span;
use std::cmp::Reverse;
use syn::spanned::Spanned;
use syn::{Attribute, Block, ImplItem, Item, TraitItem};
use tpt_weave_core::{SymbolId, SymbolKind};

/// Replacement text written in place of a removed function body.
pub const BODY_PLACEHOLDER: &str = "{ /* ... */ }";

/// A rendered source skeleton (todo.md Phase 4 "Skeleton generation").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skeleton {
    /// Path passed in; also used by `// path:line` markers.
    pub path: String,
    /// Skeleton text (parses as valid Rust).
    pub text: String,
    /// Number of function bodies removed (kept bodies are not counted).
    pub removed_bodies: u32,
}

/// Renders `source` with every function body removed.
pub fn skeleton(input: &FileInput<'_>, source: &str) -> Result<Skeleton, ParseError> {
    skeleton_with(input, source, |_| false)
}

/// Renders `source` with the bodies of symbols rejected by `keep_body`
/// removed; bodies accepted by `keep_body` are preserved verbatim.
///
/// Type definitions, attributes, impl relationships and module structure
/// are always preserved; `// path:line` markers are emitted before every
/// symbol-defining item.
pub fn skeleton_with<F>(
    input: &FileInput<'_>,
    source: &str,
    keep_body: F,
) -> Result<Skeleton, ParseError>
where
    F: FnMut(&SymbolId) -> bool,
{
    let file = syn::parse_file(source).map_err(|err| {
        let start = err.span().start();
        ParseError {
            message: err.to_string(),
            line: start.line as u32,
            column: start.column as u32 + 1,
        }
    })?;

    let line_index = LineIndex::new(source);
    let mut walker = Walker {
        input,
        source,
        line_index: &line_index,
        keep: keep_body,
        module_path: input
            .module_prefix
            .iter()
            .map(|segment| (*segment).to_string())
            .collect(),
        ops: Vec::new(),
        removed: 0,
    };
    walker.walk_items(&file.items);

    // Drop markers that would land inside a body this render removes.
    let removed_ranges: Vec<(usize, usize)> = walker
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Cut { start, end } => Some((*start, *end)),
            Op::Mark { .. } => None,
        })
        .collect();
    let mut ops = walker.ops;
    ops.retain(|op| match op {
        Op::Mark { at, .. } => !removed_ranges
            .iter()
            .any(|(start, end)| *start <= *at && *at < *end),
        Op::Cut { .. } => true,
    });

    // Apply high-offset operations first so earlier offsets stay valid.
    ops.sort_by_key(|op| Reverse(op.pos()));
    let mut text = source.to_string();
    for op in ops {
        match op {
            Op::Mark { at, text: marker } => {
                if text.is_char_boundary(at) {
                    text.insert_str(at, &marker);
                }
            }
            Op::Cut { start, end } => {
                if text.is_char_boundary(start) && text.is_char_boundary(end) {
                    text.replace_range(start..end, BODY_PLACEHOLDER);
                }
            }
        }
    }

    // Fail closed if the surgery produced something that no longer parses.
    if let Err(err) = syn::parse_file(&text) {
        let start = err.span().start();
        return Err(ParseError {
            message: format!("skeleton rendering produced invalid Rust: {err}"),
            line: start.line as u32,
            column: start.column as u32 + 1,
        });
    }

    Ok(Skeleton {
        path: input.path.to_string(),
        text,
        removed_bodies: walker.removed,
    })
}

/// One rendering operation, positioned by byte offset in the source.
enum Op {
    /// Insert a `// path:line` marker at `at`.
    Mark { at: usize, text: String },
    /// Replace `source[start..end]` with [`BODY_PLACEHOLDER`].
    Cut { start: usize, end: usize },
}

impl Op {
    fn pos(&self) -> usize {
        match self {
            Op::Mark { at, .. } => *at,
            Op::Cut { start, .. } => *start,
        }
    }
}

/// Maps 1-based line / 0-based byte-column positions to byte offsets.
struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset + 1);
            }
        }
        Self { line_starts }
    }

    fn offset(&self, line: usize, column: usize) -> Option<usize> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        Some(start + column)
    }

    /// Byte offset of the first character of a 1-based line.
    fn line_start(&self, line: usize) -> Option<usize> {
        let index = line.checked_sub(1)?;
        self.line_starts.get(index).copied()
    }
}

struct Walker<'a, F> {
    input: &'a FileInput<'a>,
    source: &'a str,
    line_index: &'a LineIndex,
    keep: F,
    module_path: Vec<String>,
    ops: Vec<Op>,
    removed: u32,
}

impl<'a, F: FnMut(&SymbolId) -> bool> Walker<'a, F> {
    fn walk_items(&mut self, items: &[Item]) {
        for item in items {
            self.walk_item(item);
        }
    }

    /// Emits a `// path:line` marker at the start of the item's first line
    /// (the attributes' line when present).
    fn mark(&mut self, attrs: &[Attribute], fallback: Span) {
        let start = attrs
            .first()
            .map(|attr| attr.span().start())
            .unwrap_or_else(|| fallback.start());
        let Some(at) = self.line_index.line_start(start.line) else {
            return;
        };
        let text = format!("// {}:{}\n", self.input.path, start.line);
        self.ops.push(Op::Mark { at, text });
    }

    /// Records the removal of `body` unless `id` is selected to be kept.
    fn cut_body(&mut self, body: &Block, id: &SymbolId) {
        let start = body.span().start();
        let end = body.span().end();
        let (Some(start_offset), Some(end_offset)) = (
            self.line_index.offset(start.line, start.column),
            self.line_index.offset(end.line, end.column),
        ) else {
            return;
        };
        if start_offset >= end_offset || end_offset > self.source.len() {
            return;
        }
        if !self.source.is_char_boundary(start_offset) || !self.source.is_char_boundary(end_offset)
        {
            return;
        }
        // Empty bodies carry no information; leave them untouched.
        if self.source[start_offset..end_offset].trim() == "{}" {
            return;
        }
        if (self.keep)(id) {
            return;
        }
        self.ops.push(Op::Cut {
            start: start_offset,
            end: end_offset,
        });
        self.removed += 1;
    }

    fn make_id(&self, kind: SymbolKind, name: String) -> SymbolId {
        SymbolId {
            repository: self.input.repository.clone(),
            package: self.input.package.to_string(),
            module: self.module_path.join("::"),
            name,
            kind,
        }
    }

    fn walk_item(&mut self, item: &Item) {
        match item {
            Item::Mod(module) => {
                self.mark(&module.attrs, module.ident.span());
                if let Some((_, items)) = &module.content {
                    self.module_path.push(module.ident.to_string());
                    self.walk_items(items);
                    self.module_path.pop();
                }
            }
            Item::Struct(st) => self.mark(&st.attrs, st.ident.span()),
            Item::Enum(en) => self.mark(&en.attrs, en.ident.span()),
            Item::Fn(fun) => {
                self.mark(&fun.attrs, fun.sig.ident.span());
                let id = self.make_id(SymbolKind::Function, fun.sig.ident.to_string());
                self.cut_body(&fun.block, &id);
            }
            Item::Const(konst) => self.mark(&konst.attrs, konst.ident.span()),
            Item::Static(stat) => self.mark(&stat.attrs, stat.ident.span()),
            Item::Type(alias) => self.mark(&alias.attrs, alias.ident.span()),
            Item::Macro(mac) => {
                if let (Some(ident), true) = (&mac.ident, mac.mac.path.is_ident("macro_rules")) {
                    self.mark(&mac.attrs, ident.span());
                }
            }
            Item::Trait(tr) => {
                self.mark(&tr.attrs, tr.ident.span());
                let trait_name = tr.ident.to_string();
                for trait_item in &tr.items {
                    if let TraitItem::Fn(method) = trait_item {
                        self.mark(&method.attrs, method.sig.ident.span());
                        let id = self.make_id(
                            SymbolKind::Method,
                            format!("{}::{}", trait_name, method.sig.ident),
                        );
                        if let Some(body) = &method.default {
                            self.cut_body(body, &id);
                        }
                    }
                }
            }
            Item::Impl(im) => {
                self.mark(&im.attrs, im.impl_token.span());
                let type_name = type_name(&im.self_ty);
                for impl_item in &im.items {
                    match impl_item {
                        ImplItem::Fn(method) => {
                            self.mark(&method.attrs, method.sig.ident.span());
                            let id = self.make_id(
                                SymbolKind::Method,
                                format!("{}::{}", type_name, method.sig.ident),
                            );
                            self.cut_body(&method.block, &id);
                        }
                        ImplItem::Const(konst) => self.mark(&konst.attrs, konst.ident.span()),
                        _ => {}
                    }
                }
            }
            // `use`, `extern crate`, foreign modules and verbatim items are
            // not symbol definitions and get no marker.
            _ => {}
        }
    }
}
