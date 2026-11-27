//! `glossary`: content kinds use the project's vocabulary, never a term GLOSSARY.md avoids.
//!
//! The names checked, against every term on an `_Avoid_:` line of the staged GLOSSARY.md:
//! - `kind`: the `<kind>` segment of a Content ID (`<ruleset>:<kind>/…`) in any file;
//! - `kind-name`: the string literal a `const NAME` in a Rust trait impl gives: the `Kind`
//!   impls, under whatever name the trait is imported as;
//! - `variant`: a variant of a Rust `enum Record` or `enum Encoding`, and every string a
//!   `rename` or `alias` gives it, in either form, in a `#[serde(…)]` attribute or one inside
//!   `#[cfg_attr(…)]`, since RON reads a pack in those names.
//!
//! A name and a term match when their letters and digits agree, ignoring case, spaces,
//! hyphens and underscores, so `MasteryProperty`, `mastery-property` and "mastery property"
//! match, as do `Dps` and "DPS", and `Level20Snapshot` and "level 20 snapshot".
//!
//! Rust files are parsed with `syn`, so comments, strings, attributes and generics cannot hide
//! a name. What the guard cannot read is a `parse` problem, never a silent pass: a Rust file
//! that does not parse, a `const NAME` in a trait impl that is not a string literal, or a serde
//! attribute on a content variant that does not parse. Items a macro generates are not seen;
//! today every `Kind` impl is written out, and a repository test fails if fewer are found.
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
use syn::punctuated::Punctuated;
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

#[derive(Debug, Clone, Copy)]
enum Check {
    /// A Content ID kind.
    Kind,
    /// A `const NAME` in a trait impl.
    KindName,
    /// A content enum variant, or a serde name on it.
    Variant,
    /// Something the guard cannot read, so cannot check.
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

/// What a found name is.
#[derive(Debug, Clone, Copy)]
enum Named {
    ContentIdKind,
    KindName,
    Variant,
}

impl Named {
    fn check(self) -> Check {
        match self {
            Named::ContentIdKind => Check::Kind,
            Named::KindName => Check::KindName,
            Named::Variant => Check::Variant,
        }
    }

    /// How a report calls it.
    fn noun(self) -> &'static str {
        match self {
            Named::ContentIdKind | Named::KindName => "kind",
            Named::Variant => "variant",
        }
    }
}

/// A term GLOSSARY.md says to avoid, and the entry that says so.
#[derive(Debug)]
struct Avoided {
    /// The term's [`key`].
    key: String,
    entry: String,
}

/// A name the guard checks, where it was found.
#[derive(Debug)]
struct Found {
    /// The line index, from 0.
    line: usize,
    named: Named,
    name: String,
}

/// Something the guard cannot read, where it is.
#[derive(Debug)]
struct Unreadable {
    /// The line index, from 0.
    line: usize,
    message: String,
}

/// How many names of each sort a scan read.
#[derive(Debug, Default)]
struct Counts {
    kinds: usize,
    kind_names: usize,
    variants: usize,
}

impl Counts {
    fn add(&mut self, named: Named) {
        match named {
            Named::ContentIdKind => self.kinds += 1,
            Named::KindName => self.kind_names += 1,
            Named::Variant => self.variants += 1,
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
        let mut unreadable = Vec::new();
        if Path::new(&file.path)
            .extension()
            .is_some_and(|ext| ext == "rs")
        {
            let names = rust_names(&text);
            found.extend(names.found);
            unreadable = names.unreadable;
        }
        for problem in unreadable {
            report.push(Check::Parse.problem(at_line(&file.path, problem.line), problem.message));
        }
        for name in found {
            counts.add(name.named);
            let name_key = key(&name.name);
            if let Some(hit) = avoided.iter().find(|a| a.key == name_key) {
                report.push(name.named.check().problem(
                    at_line(&file.path, name.line),
                    format!(
                        "the {} \"{}\" is a term {GLOSSARY} avoids (see **{}**)",
                        name.named.noun(),
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

/// What a name and a term are compared by: their letters and digits, lowercase.
fn key(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The kind of every Content ID in `text`.
fn content_id_kinds(content_id: &Regex, text: &str) -> Vec<Found> {
    text.lines()
        .enumerate()
        .flat_map(|(line, text)| {
            content_id
                .captures_iter(text)
                .filter_map(|caps| caps.name("kind"))
                .map(move |kind| Found {
                    line,
                    named: Named::ContentIdKind,
                    name: kind.as_str().to_owned(),
                })
        })
        .collect()
}

/// Every `const NAME` in a trait impl and content enum variant in Rust `text`, and what
/// could not be read.
#[derive(Debug, Default)]
struct RustNames {
    found: Vec<Found>,
    unreadable: Vec<Unreadable>,
}

fn rust_names(text: &str) -> RustNames {
    let mut names = RustNames::default();
    match syn::parse_file(text) {
        Ok(file) => names.visit_file(&file),
        Err(err) => names.unreadable(err.span(), &format!("cannot parse the file: {err}")),
    }
    names
}

/// The line index of a span's start.
fn line_of(at: proc_macro2::Span) -> usize {
    at.start().line.saturating_sub(1)
}

impl RustNames {
    fn push(&mut self, at: proc_macro2::Span, named: Named, name: String) {
        self.found.push(Found {
            line: line_of(at),
            named,
            name,
        });
    }

    fn unreadable(&mut self, at: proc_macro2::Span, message: &str) {
        self.unreadable.push(Unreadable {
            line: line_of(at),
            message: message.to_owned(),
        });
    }
}

impl<'ast> Visit<'ast> for RustNames {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if item.trait_.is_some() {
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
                    }) => self.push(name.span(), Named::KindName, name.value()),
                    _ => self.unreadable(
                        constant.ident.span(),
                        "cannot check a `const NAME` that is not a string literal",
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
                    Named::Variant,
                    variant.ident.to_string(),
                );
                for attr in &variant.attrs {
                    match serde_names(&attr.meta) {
                        Ok(names) => {
                            for name in names {
                                self.push(name.span(), Named::Variant, name.value());
                            }
                        }
                        Err(err) => self.unreadable(
                            err.span(),
                            &format!(
                                "cannot check the serde attribute on {}: {err}",
                                variant.ident
                            ),
                        ),
                    }
                }
            }
        }
        syn::visit::visit_item_enum(self, item);
    }
}

/// The names an attribute gives a variant in RON: `rename` and `alias` in a `serde(…)`, in
/// their `= "…"` and `(serialize = "…", deserialize = "…")` forms, including a `serde(…)`
/// inside `cfg_attr(…)`. Every other entry is read past, so one cannot hide a later name.
fn serde_names(meta: &syn::Meta) -> syn::Result<Vec<syn::LitStr>> {
    let mut names = Vec::new();
    if meta.path().is_ident("cfg_attr") {
        let inner = meta
            .require_list()?
            .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)?;
        // The first entry is the condition; the rest are attributes.
        for attr in inner.iter().skip(1) {
            names.extend(serde_names(attr)?);
        }
    } else if meta.path().is_ident("serde") {
        meta.require_list()?.parse_nested_meta(|entry| {
            let names_it = entry.path.is_ident("rename") || entry.path.is_ident("alias");
            if entry.input.peek(syn::Token![=]) {
                let value = entry.value()?;
                if names_it {
                    names.push(value.parse()?);
                } else {
                    let _: syn::Expr = value.parse()?;
                }
            } else if entry.input.peek(syn::token::Paren) {
                if names_it {
                    entry.parse_nested_meta(|side| {
                        names.push(side.value()?.parse()?);
                        Ok(())
                    })?;
                } else {
                    let content;
                    syn::parenthesized!(content in entry.input);
                    let _: proc_macro2::TokenStream = content.parse()?;
                }
            }
            Ok(())
        })?;
    }
    Ok(names)
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
                    .map(key)
                    .filter(|key| !key.is_empty())
                    .map(|key| Avoided {
                        key,
                        entry: entry.to_owned(),
                    }),
            );
        }
    }
    avoided
}
