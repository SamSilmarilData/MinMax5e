//! Loads records and RON encodings into a Rules Pack for one Ruleset (architecture §8).
//!
//! The public seam:
//! - [`load`]: [`Sources`] → [`Pack<R>`](Pack), or a [`LoadError`] holding one
//!   [`Diagnostic`] per problem, each naming its file and pointing at the offending text.
//! - [`Pack`]: the read-only pack, which only `load` can construct. Look items up by
//!   Content ID to get a typed [`Id`], then index the pack with it.
//! - [`Pack::coverage`]: which in-scope records have an encoding and which do not, so
//!   unencoded content is never silently ignored (ADR-0002).
//!
//! Every Content ID must carry the Ruleset's prefix, so an ID from another edition can
//! never load (ADR-0008).

use std::marker::PhantomData;
use std::ops::Index;

use miette::{NamedSource, SourceSpan};
use mm_core::effect::{ClassEncoding, Encoding};
use mm_core::kind::{self, Kind};
use mm_core::records::{ClassRecord, Record, WeaponRecord};
use mm_core::{ContentId, Id, Ruleset, Source, SourceRole, Sources};
use rustc_hash::FxHashMap;

/// A loaded Rules Pack for Ruleset `R`: every record, with its encoding where it has one.
#[derive(Debug)]
pub struct Pack<R> {
    classes: Vec<ClassDef>,
    weapons: Vec<WeaponRecord>,
    by_id: FxHashMap<ContentId, Entry>,
    ruleset: PhantomData<fn() -> R>,
}

/// A class: its record and, once encoded, its mechanics.
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub record: ClassRecord,
    pub encoding: Option<ClassEncoding>,
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    Class(u32),
    Weapon(u32),
}

/// Which in-scope records a pack encodes (ADR-0002), each list in Content ID order.
///
/// A record is in scope when its mechanics live in an encoding: classes, for now. A
/// weapon's mechanics are its record's imported fields (ADR-0002 amendment), so weapons
/// need no encoding and are not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub encoded: Vec<ContentId>,
    pub missing: Vec<ContentId>,
}

/// Loading failed. Each diagnostic is one problem.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("the pack has {} problem(s)", diagnostics.len())]
pub struct LoadError {
    #[related]
    pub diagnostics: Vec<Diagnostic>,
}

/// One problem in one source file, with the span it is about.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{message}")]
pub struct Diagnostic {
    pub message: String,
    #[source_code]
    pub file: NamedSource<String>,
    #[label("here")]
    pub span: SourceSpan,
}

/// Loads `sources` into a pack for Ruleset `R`.
///
/// # Errors
/// If any source fails to parse, any Content ID lacks `R`'s prefix or names the wrong
/// kind, an ID is defined twice, or an encoding names no loaded record. Every problem
/// found is reported, not just the first.
pub fn load<R: Ruleset>(sources: &Sources) -> Result<Pack<R>, LoadError> {
    let mut loader = Loader::<R> {
        pack: Pack {
            classes: Vec::new(),
            weapons: Vec::new(),
            by_id: FxHashMap::default(),
            ruleset: PhantomData,
        },
        diagnostics: Vec::new(),
    };
    // Records first, so encodings can find what they encode whatever the file order.
    for role in [SourceRole::Records, SourceRole::Encoding] {
        for source in sources.files.iter().filter(|s| s.role == role) {
            loader.source(source);
        }
    }
    if loader.diagnostics.is_empty() {
        Ok(loader.pack)
    } else {
        Err(LoadError {
            diagnostics: loader.diagnostics,
        })
    }
}

impl<R: Ruleset> Pack<R> {
    /// The class with this Content ID, if the pack has one.
    pub fn class(&self, id: &ContentId) -> Option<Id<R, kind::Class>> {
        match self.by_id.get(id) {
            Some(&Entry::Class(i)) => Some(Id::from_index(i)),
            _ => None,
        }
    }

    /// The weapon with this Content ID, if the pack has one.
    pub fn weapon(&self, id: &ContentId) -> Option<Id<R, kind::Item>> {
        match self.by_id.get(id) {
            Some(&Entry::Weapon(i)) => Some(Id::from_index(i)),
            _ => None,
        }
    }
}

impl<R> Pack<R> {
    /// Which in-scope records have an encoding and which do not.
    pub fn coverage(&self) -> Coverage {
        let (encoded, missing): (Vec<&ClassDef>, Vec<&ClassDef>) =
            self.classes.iter().partition(|def| def.encoding.is_some());
        let sorted_ids = |defs: Vec<&ClassDef>| {
            let mut ids: Vec<ContentId> = defs.into_iter().map(|d| d.record.id.clone()).collect();
            ids.sort();
            ids
        };
        Coverage {
            encoded: sorted_ids(encoded),
            missing: sorted_ids(missing),
        }
    }
}

impl<R: Ruleset> Index<Id<R, kind::Class>> for Pack<R> {
    type Output = ClassDef;

    fn index(&self, id: Id<R, kind::Class>) -> &ClassDef {
        &self.classes[id.index() as usize]
    }
}

impl<R: Ruleset> Index<Id<R, kind::Item>> for Pack<R> {
    type Output = WeaponRecord;

    fn index(&self, id: Id<R, kind::Item>) -> &WeaponRecord {
        &self.weapons[id.index() as usize]
    }
}

struct Loader<R> {
    pack: Pack<R>,
    diagnostics: Vec<Diagnostic>,
}

impl<R: Ruleset> Loader<R> {
    fn source(&mut self, source: &Source) {
        match source.role {
            SourceRole::Records => {
                if let Some(records) = self.parse::<Vec<Record>>(source) {
                    for record in records {
                        self.record(source, record);
                    }
                }
            }
            SourceRole::Encoding => {
                if let Some(encoding) = self.parse::<Encoding>(source) {
                    self.encoding(source, encoding);
                }
            }
        }
    }

    fn parse<T: serde::de::DeserializeOwned>(&mut self, source: &Source) -> Option<T> {
        match ron::Options::default().from_str(&source.text) {
            Ok(value) => Some(value),
            Err(error) => {
                let start = offset(&source.text, error.span.start);
                let end = offset(&source.text, error.span.end).max(start);
                self.report(source, (start, end - start), error.code.to_string());
                None
            }
        }
    }

    fn record(&mut self, source: &Source, record: Record) {
        let (entry, wanted) = match &record {
            Record::Class(_) => (Entry::Class(len(&self.pack.classes)), kind::Class::NAME),
            Record::Weapon(_) => (Entry::Weapon(len(&self.pack.weapons)), kind::Item::NAME),
        };
        let id = record.id().clone();
        if !self.check_id(source, &id, wanted) {
            return;
        }
        if self.pack.by_id.contains_key(&id) {
            self.report_id(source, &id, format!("{id} is defined twice"));
            return;
        }
        self.pack.by_id.insert(id, entry);
        match record {
            Record::Class(record) => self.pack.classes.push(ClassDef {
                record,
                encoding: None,
            }),
            Record::Weapon(record) => self.pack.weapons.push(record),
        }
    }

    fn encoding(&mut self, source: &Source, encoding: Encoding) {
        match encoding {
            Encoding::Class(encoding) => {
                let id = encoding.class.clone();
                if !self.check_id(source, &id, kind::Class::NAME) {
                    return;
                }
                let Some(class) = self.pack.class(&id) else {
                    self.report_id(source, &id, format!("{id} encodes no loaded class record"));
                    return;
                };
                let def = &mut self.pack.classes[class.index() as usize];
                if def.encoding.is_some() {
                    self.report_id(source, &id, format!("{id} is encoded twice"));
                    return;
                }
                def.encoding = Some(encoding);
            }
        }
    }

    /// Whether `id` has the Ruleset's prefix and the `wanted` kind; reports it if not.
    fn check_id(&mut self, source: &Source, id: &ContentId, wanted: &str) -> bool {
        if id.ruleset() != R::PREFIX {
            self.report_id(
                source,
                id,
                format!(
                    "{id} belongs to Ruleset `{}`, but this pack is `{}` (ADR-0008)",
                    id.ruleset(),
                    R::PREFIX
                ),
            );
            false
        } else if id.kind() != wanted {
            self.report_id(
                source,
                id,
                format!("{id} is a `{}`, but a `{wanted}` belongs here", id.kind()),
            );
            false
        } else {
            true
        }
    }

    /// Reports a problem with `id`, pointing at its first quoted occurrence in the source.
    fn report_id(&mut self, source: &Source, id: &ContentId, message: String) {
        let quoted = format!("\"{id}\"");
        let span = source
            .text
            .find(&quoted)
            .map_or((0, 0), |at| (at, quoted.len()));
        self.report(source, span, message);
    }

    fn report(&mut self, source: &Source, span: (usize, usize), message: String) {
        self.diagnostics.push(Diagnostic {
            message,
            file: NamedSource::new(&source.name, source.text.clone()),
            span: span.into(),
        });
    }
}

fn len<T>(items: &[T]) -> u32 {
    u32::try_from(items.len()).expect("a pack holds fewer than 2³² items of a kind")
}

/// The byte offset of a 1-based line and column (in characters) in `text`, clamped to its end.
fn offset(text: &str, at: ron::error::Position) -> usize {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(at.line.saturating_sub(1))
        .map(str::len)
        .sum();
    let line = &text[line_start.min(text.len())..];
    line_start
        + line
            .char_indices()
            .nth(at.col.saturating_sub(1))
            .map_or(line.len(), |(i, _)| i)
}
