//! `glossary`: content kinds use the project's vocabulary, never a term GLOSSARY.md avoids.
//!
//! The names checked, each read as lowercase words, against every term on an `_Avoid_:`
//! line of the staged GLOSSARY.md:
//! - `kind`: the `<kind>` segment of a Content ID (`<ruleset>:<kind>/…`) in any file, hyphens
//!   read as spaces;
//! - `kind-name`: the string literal a Rust `impl Kind for … { const NAME … }` gives;
//! - `variant`: a variant of a Rust `enum Record` or `enum Encoding`, split at its capitals
//!   (`MasteryProperty` is "mastery property"), and every string a `#[serde(rename …)]` or
//!   `#[serde(alias …)]` on it gives, in either form, since RON reads a pack in those names.
//!
//! Rust files are parsed with `syn`, so comments, strings, attributes and generics cannot hide
//! a name. What the guard cannot read is a `parse` problem, never a silent pass: a Rust file
//! that does not parse, a `Kind::NAME` that is not a string literal, or a serde attribute on a
//! content variant that does not parse.
//!
//! Prose and other identifiers are left to review: the avoid lists hold common words
//! ("source", "version"), and terms one entry avoids that another defines ("corpus", "errata"),
//! which a wider scan would flag everywhere.
//!
//! Research notes stay as written (`CODING_STANDARDS.md`, "Research errors go to the errata"),
//! so files under `docs/research/` are not scanned.

use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;
use syn::visit::Visit;

use crate::Outcome;
use crate::content_id;
use crate::git::tracked_files;
use crate::report::{Check as _, Report, at_line};

const GLOSSARY: &str = "GLOSSARY.md";

/// Files under this directory are not scanned.
const RESEARCH_DIR: &str = "docs/research/";

/// The enums whose variant names a pack's RON is written in.
const CONTENT_ENUMS: [&str; 2] = ["Record", "Encoding"];

/// The trait whose `NAME` is a Content ID kind.
const KIND_TRAIT: &str = "Kind";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    /// A Content ID kind.
    Kind,
    /// A `Kind::NAME` in Rust.
    KindName,
    /// A content enum variant, or its serde rename.
    Variant,
    /// A Rust file `syn` cannot parse, so its names cannot be checked.
    Parse,
}

impl crate::report::Check for Check {
    fn name(self) -> &'static str {
        match self {
            Check::Kind => "kind",
            Check::KindName => "kind-name",
            Check::Variant => "variant",
            Check::Parse => "parse",
        }
    }
}

impl Check {
    /// What a name this check reads is, in a report.
    fn noun(self) -> &'static str {
        match self {
            Check::Kind | Check::KindName => "kind",
            Check::Variant => "variant",
            Check::Parse => "name",
        }
    }
}

/// A term GLOSSARY.md says to avoid, and the entry that says so.
#[derive(Debug)]
struct Avoided {
    /// Lowercase, without any parenthetical note.
    term: String,
    entry: String,
}

/// A name the guard checks, where it was found.
#[derive(Debug)]
struct Found {
    /// The line index, from 0.
    line: usize,
    check: Check,
    name: String,
    /// The name as lowercase words, as the avoid lists write terms.
    words: String,
}

impl Found {
    fn new(line: usize, check: Check, name: String) -> Self {
        let words = words_of(&name);
        Self {
            line,
            check,
            name,
            words,
        }
    }
}

/// How many names of each check a scan read.
#[derive(Debug, Default)]
struct Counts {
    kinds: usize,
    kind_names: usize,
    variants: usize,
}

impl Counts {
    fn add(&mut self, check: Check) {
        match check {
            Check::Kind => self.kinds += 1,
            Check::KindName => self.kind_names += 1,
            Check::Variant => self.variants += 1,
            Check::Parse => {}
        }
    }
}

pub fn run(root: &Path) -> Result<Outcome> {
    let files = tracked_files(root)?;
    let glossary = files
        .iter()
        .find(|file| file.path == GLOSSARY)
        .with_context(|| format!("{GLOSSARY} is not tracked in {}", root.display()))?;
    let avoided = avoided_terms(&String::from_utf8_lossy(&glossary.contents));

    let content_id = Regex::new(content_id::PATTERN).expect("valid regex");
    let mut report = Report::new("glossary");
    let mut counts = Counts::default();
    let mut scanned = 0;
    for file in files.iter().filter(|f| !f.path.starts_with(RESEARCH_DIR)) {
        scanned += 1;
        let text = String::from_utf8_lossy(&file.contents);
        let mut found = content_id_kinds(&content_id, &text);
        if Path::new(&file.path)
            .extension()
            .is_some_and(|ext| ext == "rs")
        {
            match syn::parse_file(&text) {
                Ok(parsed) => found.extend(rust_names(&parsed)),
                Err(err) => report.push(Check::Parse.problem(
                    at_line(&file.path, err.span().start().line.saturating_sub(1)),
                    format!("cannot parse it to check its names: {err}"),
                )),
            }
        }
        for name in found {
            counts.add(name.check);
            if name.check == Check::Parse {
                report.push(
                    name.check
                        .problem(at_line(&file.path, name.line), name.name),
                );
                continue;
            }
            if let Some(hit) = avoided.iter().find(|a| a.term == name.words) {
                report.push(name.check.problem(
                    at_line(&file.path, name.line),
                    format!(
                        "the {} \"{}\" is a term {GLOSSARY} avoids (see **{}**)",
                        name.check.noun(),
                        name.name,
                        hit.entry
                    ),
                ));
            }
        }
    }
    // The counts show the scan found names at all: a renamed trait or content enum would
    // otherwise pass silently.
    let Counts {
        kinds,
        kind_names,
        variants,
    } = counts;
    Ok(report.finish(
        &format!(
            "{kinds} Content ID kind(s), {kind_names} Kind::NAME(s) and {variants} content enum \
             variant(s) in {scanned} tracked file(s) clean against {} avoided term(s)",
            avoided.len()
        ),
        "Name it after the glossary entry it belongs to.",
    ))
}

/// The kind of every Content ID in `text`.
fn content_id_kinds(content_id: &Regex, text: &str) -> Vec<Found> {
    text.lines()
        .enumerate()
        .flat_map(|(index, line)| {
            content_id
                .captures_iter(line)
                .filter_map(|caps| caps.name("kind"))
                .map(move |kind| Found::new(index, Check::Kind, kind.as_str().to_owned()))
        })
        .collect()
}

/// Every `Kind::NAME` and content enum variant in a parsed Rust file.
fn rust_names(file: &syn::File) -> Vec<Found> {
    let mut names = RustNames::default();
    names.visit_file(file);
    names.found
}

#[derive(Default)]
struct RustNames {
    found: Vec<Found>,
}

impl RustNames {
    fn push(&mut self, at: proc_macro2::Span, check: Check, name: String) {
        let line = at.start().line.saturating_sub(1);
        self.found.push(Found::new(line, check, name));
    }

    /// A name this cannot read, so cannot check: a `parse` problem, never a silent pass.
    fn unreadable(&mut self, at: proc_macro2::Span, what: &str) {
        self.push(at, Check::Parse, format!("cannot check {what}"));
    }
}

impl<'ast> Visit<'ast> for RustNames {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let is_kind = item
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path.segments.last())
            .is_some_and(|segment| segment.ident == KIND_TRAIT);
        if is_kind {
            for member in &item.items {
                let syn::ImplItem::Const(constant) = member else {
                    continue;
                };
                if constant.ident != "NAME" {
                    continue;
                }
                match &constant.expr {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(name),
                        ..
                    }) => self.push(name.span(), Check::KindName, name.value()),
                    _ => self.unreadable(
                        constant.ident.span(),
                        "a Kind::NAME that is not a string literal",
                    ),
                }
            }
        }
        syn::visit::visit_item_impl(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        if CONTENT_ENUMS.iter().any(|name| item.ident == name) {
            for variant in &item.variants {
                self.push(
                    variant.ident.span(),
                    Check::Variant,
                    variant.ident.to_string(),
                );
                for attr in variant.attrs.iter().filter(|a| a.path().is_ident("serde")) {
                    match serde_names(attr) {
                        Ok(names) => {
                            for name in names {
                                self.push(name.span(), Check::Variant, name.value());
                            }
                        }
                        Err(err) => self.unreadable(
                            err.span(),
                            &format!("the serde attribute on {}: {err}", variant.ident),
                        ),
                    }
                }
            }
        }
        syn::visit::visit_item_enum(self, item);
    }
}

/// The names a `#[serde(…)]` attribute gives a variant in RON: `rename` and `alias`, in
/// their `= "…"` and `(serialize = "…", deserialize = "…")` forms. Every other entry is read
/// past, so one cannot hide a later name.
fn serde_names(attr: &syn::Attribute) -> syn::Result<Vec<syn::LitStr>> {
    let mut names = Vec::new();
    attr.parse_nested_meta(|meta| {
        let names_it = meta.path.is_ident("rename") || meta.path.is_ident("alias");
        if meta.input.peek(syn::Token![=]) {
            let value = meta.value()?;
            if names_it {
                names.push(value.parse()?);
            } else {
                let _: syn::Expr = value.parse()?;
            }
        } else if meta.input.peek(syn::token::Paren) {
            if names_it {
                meta.parse_nested_meta(|side| {
                    names.push(side.value()?.parse()?);
                    Ok(())
                })?;
            } else {
                let content;
                syn::parenthesized!(content in meta.input);
                let _: proc_macro2::TokenStream = content.parse()?;
            }
        }
        Ok(())
    })?;
    Ok(names)
}

/// A name as lowercase words: `MasteryProperty`, `mastery-property` and `mastery_property`
/// are all "mastery property".
fn words_of(name: &str) -> String {
    let mut words = String::new();
    for (i, c) in name.chars().enumerate() {
        if c == '-' || c == '_' {
            words.push(' ');
            continue;
        }
        if c.is_ascii_uppercase() && i > 0 && !words.ends_with(' ') {
            words.push(' ');
        }
        words.push(c.to_ascii_lowercase());
    }
    words
}

/// Every term on an `_Avoid_:` line, with the bold entry above it.
fn avoided_terms(glossary: &str) -> Vec<Avoided> {
    let parenthetical = Regex::new(r"\s*\([^)]*\)").expect("valid regex");
    let mut entry = "";
    let mut avoided = Vec::new();
    for line in glossary.lines() {
        if let Some(name) = line.strip_prefix("**").and_then(|l| l.split("**").next()) {
            entry = name.trim_end_matches(':');
        } else if let Some(terms) = line.strip_prefix("_Avoid_:") {
            let terms = parenthetical.replace_all(terms, "");
            avoided.extend(
                terms
                    .split(',')
                    .map(|term| term.trim().to_lowercase())
                    .filter(|term| !term.is_empty())
                    .map(|term| Avoided {
                        term,
                        entry: entry.to_owned(),
                    }),
            );
        }
    }
    avoided
}
