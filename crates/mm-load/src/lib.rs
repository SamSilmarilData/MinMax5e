//! Loads records and RON encodings into a Rules Pack for one Ruleset (architecture §8).
//!
//! The public seam:
//! - [`load`]: [`Sources`] → [`Pack<R>`](Pack), or a [`LoadError`] holding one
//!   [`Diagnostic`] per problem, each naming its file and pointing at the offending text.
//! - [`Pack`]: the read-only pack, which only `load` can construct. Look items up by
//!   Content ID to get a typed [`Id`], then index the pack with it.
//! - [`Pack::coverage`]: which in-scope records have an encoding and which do not, so
//!   unencoded content is never silently ignored (ADR-0002).
//! - [`Pack::ruling`]: the Rulings the pack declares (ADR-0006). Each is checked when it
//!   loads (two or more distinct options, a default among them, a citation, declared
//!   once), and so is
//!   every Ruling an encoding's predicate reads, with its option.
//!
//! Every Content ID must carry the Ruleset's prefix, so an ID from another edition can
//! never load (ADR-0008).

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Index;

use miette::{NamedSource, SourceSpan};
use mm_core::effect::{ClassEncoding, Encoding, FeaturesEncoding, Pred, Rider};
use mm_core::kind::{self, Kind};
use mm_core::records::{ClassRecord, NamedRecord, Record, WeaponRecord};
use mm_core::{
    ContentId, Id, Ruleset, Ruling, RulingError, RulingId, RulingOptionId, Source, SourceRole,
    Sources,
};
use rustc_hash::FxHashMap;

/// A loaded Rules Pack for Ruleset `R`: every record, with its encoding where it has one.
#[derive(Debug)]
pub struct Pack<R> {
    classes: Vec<ClassDef>,
    /// Species, backgrounds, feats and mastery properties, told apart by their Content ID's kind.
    feature_defs: Vec<FeaturesDef>,
    weapons: Vec<WeaponRecord>,
    by_id: FxHashMap<ContentId, Entry>,
    rulings: BTreeMap<RulingId, Ruling>,
    ruleset: PhantomData<fn() -> R>,
}

/// A class: its record and, once encoded, its mechanics.
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub record: ClassRecord,
    pub encoding: Option<ClassEncoding>,
}

/// A species, background, feat or mastery property: its record and, once encoded, its mechanics.
#[derive(Debug, Clone)]
pub struct FeaturesDef {
    pub record: NamedRecord,
    pub encoding: Option<FeaturesEncoding>,
}

/// The kinds whose records are [`FeaturesDef`]s: species, backgrounds, feats and mastery
/// properties.
pub trait FeaturesKind: Kind {}

impl FeaturesKind for kind::Species {}
impl FeaturesKind for kind::Background {}
impl FeaturesKind for kind::Feat {}
impl FeaturesKind for kind::MasteryProperty {}

#[derive(Debug, Clone, Copy)]
enum Entry {
    Class(u32),
    FeaturesDef(u32),
    Weapon(u32),
}

/// Which in-scope records a pack encodes (ADR-0002), each list in Content ID order.
///
/// A record is in scope when its mechanics live in an encoding: classes, species,
/// backgrounds, feats and mastery properties. A weapon's mechanics are its record's imported fields
/// (ADR-0002 amendment), so weapons need no encoding and are not listed.
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
/// kind, an ID is defined twice, an encoding names no loaded record, or a weapon names a
/// mastery property the pack lacks; if a Ruling is declared twice or is malformed, or an
/// encoding reads a Ruling or option the pack lacks. Every problem found is reported, not
/// just the first.
pub fn load<R: Ruleset>(sources: &Sources) -> Result<Pack<R>, LoadError> {
    let mut loader = Loader::<'_, R> {
        pack: Pack {
            classes: Vec::new(),
            feature_defs: Vec::new(),
            weapons: Vec::new(),
            by_id: FxHashMap::default(),
            rulings: BTreeMap::new(),
            ruleset: PhantomData,
        },
        diagnostics: Vec::new(),
        mastery_property_refs: Vec::new(),
    };
    // Records and Rulings first, so encodings can find what they encode and read whatever
    // the file order.
    for role in [
        SourceRole::Records,
        SourceRole::Ruling,
        SourceRole::Encoding,
    ] {
        for source in sources.files.iter().filter(|s| s.role == role) {
            loader.source(source);
        }
    }
    loader.check_mastery_properties();
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

    /// The species, background, feat or mastery property with this Content ID, if the pack has one
    /// of kind `K`.
    pub fn features_def<K: FeaturesKind>(&self, id: &ContentId) -> Option<Id<R, K>> {
        match self.by_id.get(id) {
            Some(&Entry::FeaturesDef(i)) if id.kind() == K::NAME => Some(Id::from_index(i)),
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

    /// The Ruling with this id, if the pack declares one.
    pub fn ruling(&self, id: &RulingId) -> Option<&Ruling> {
        self.rulings.get(id)
    }

    /// Whether the pack declares Ruling `id` with option `option`.
    ///
    /// # Errors
    /// If it declares no such Ruling, or the Ruling offers no such option.
    pub fn check_ruling(&self, id: &RulingId, option: &RulingOptionId) -> Result<(), RulingError> {
        self.ruling(id)
            .ok_or_else(|| RulingError::Unknown(id.clone()))?
            .check_option(option)
    }
}

impl<R> Pack<R> {
    /// Which in-scope records have an encoding and which do not.
    pub fn coverage(&self) -> Coverage {
        let classes = self
            .classes
            .iter()
            .map(|def| (&def.record.id, def.encoding.is_some()));
        let feature_defs = self
            .feature_defs
            .iter()
            .map(|def| (&def.record.id, def.encoding.is_some()));
        let mut coverage = Coverage {
            encoded: Vec::new(),
            missing: Vec::new(),
        };
        for (id, encoded) in classes.chain(feature_defs) {
            if encoded {
                coverage.encoded.push(id.clone());
            } else {
                coverage.missing.push(id.clone());
            }
        }
        coverage.encoded.sort();
        coverage.missing.sort();
        coverage
    }
}

impl<R: Ruleset> Index<Id<R, kind::Class>> for Pack<R> {
    type Output = ClassDef;

    fn index(&self, id: Id<R, kind::Class>) -> &ClassDef {
        &self.classes[id.index() as usize]
    }
}

impl<R: Ruleset, K: FeaturesKind> Index<Id<R, K>> for Pack<R> {
    type Output = FeaturesDef;

    fn index(&self, id: Id<R, K>) -> &FeaturesDef {
        &self.feature_defs[id.index() as usize]
    }
}

impl<R: Ruleset> Index<Id<R, kind::Item>> for Pack<R> {
    type Output = WeaponRecord;

    fn index(&self, id: Id<R, kind::Item>) -> &WeaponRecord {
        &self.weapons[id.index() as usize]
    }
}

struct Loader<'s, R> {
    pack: Pack<R>,
    diagnostics: Vec<Diagnostic>,
    /// Each loaded weapon's mastery property, checked once every record is loaded.
    mastery_property_refs: Vec<MasteryPropertyRef<'s>>,
}

/// A weapon's mastery property, and the file that names it.
struct MasteryPropertyRef<'s> {
    source: &'s Source,
    weapon: ContentId,
    property: ContentId,
}

impl<'s, R: Ruleset> Loader<'s, R> {
    fn source(&mut self, source: &'s Source) {
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
                    self.check_ruling_reads(source, &encoding);
                    self.encoding(source, encoding);
                }
            }
            SourceRole::Ruling => {
                if let Some(ruling) = self.parse::<Ruling>(source) {
                    self.ruling(source, ruling);
                }
            }
        }
    }

    /// Adds a Ruling's declaration, if it is well formed and not declared already.
    fn ruling(&mut self, source: &Source, ruling: Ruling) {
        let id = ruling.id.as_str();
        if self.pack.rulings.contains_key(&ruling.id) {
            self.report_quoted(source, id, format!("Ruling `{id}` is declared twice"));
            return;
        }
        let problem = if ruling.options.len() < 2 {
            Some((id, "a Ruling offers at least two options".to_owned()))
        } else if let Some(twice) = ruling.options.iter().enumerate().find_map(|(i, o)| {
            let earlier = &ruling.options[..i];
            earlier.iter().any(|before| before.id == o.id).then_some(o)
        }) {
            // Point at the repeat, the option id's second quoted occurrence.
            let message = format!("option `{}` is offered twice", twice.id);
            self.report_quoted_nth(source, twice.id.as_str(), 1, message);
            return;
        } else if let Err(error) = ruling.check_option(&ruling.default) {
            Some((
                ruling.default.as_str(),
                format!("the default is not an option: {error}"),
            ))
        } else if ruling.citations.is_empty() {
            Some((
                id,
                "a Ruling cites at least one source (ADR-0006)".to_owned(),
            ))
        } else {
            None
        };
        if let Some((quoted, message)) = problem {
            self.report_quoted(source, quoted, message);
            return;
        }
        self.pack.rulings.insert(ruling.id.clone(), ruling);
    }

    /// Reports each Ruling an encoding's predicates read that the pack does not declare,
    /// or whose option it does not offer.
    fn check_ruling_reads(&mut self, source: &Source, encoding: &Encoding) {
        let riders: Vec<&Rider> = match encoding {
            Encoding::Class(class) => class.features.iter().flat_map(|f| &f.riders).collect(),
            Encoding::Species(e)
            | Encoding::Background(e)
            | Encoding::Feat(e)
            | Encoding::MasteryProperty(e) => e.features.iter().flat_map(|f| &f.riders).collect(),
        };
        let mut reads = Vec::new();
        for pred in riders.iter().filter_map(|rider| rider.when.as_ref()) {
            ruling_reads(pred, &mut reads);
        }
        for (id, option) in reads {
            match self.pack.check_ruling(id, option) {
                Err(error @ RulingError::Unknown(_)) => {
                    self.report_quoted(source, id.as_str(), error.to_string());
                }
                Err(error @ RulingError::UnknownOption { .. }) => {
                    self.report_quoted(source, option.as_str(), error.to_string());
                }
                Ok(()) => {}
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

    fn record(&mut self, source: &'s Source, record: Record) {
        let features_def = Entry::FeaturesDef(len(&self.pack.feature_defs));
        let (entry, wanted) = match &record {
            Record::Class(_) => (Entry::Class(len(&self.pack.classes)), kind::Class::NAME),
            Record::Species(_) => (features_def, kind::Species::NAME),
            Record::Background(_) => (features_def, kind::Background::NAME),
            Record::Feat(_) => (features_def, kind::Feat::NAME),
            Record::MasteryProperty(_) => (features_def, kind::MasteryProperty::NAME),
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
            Record::Species(record)
            | Record::Background(record)
            | Record::Feat(record)
            | Record::MasteryProperty(record) => self.pack.feature_defs.push(FeaturesDef {
                record,
                encoding: None,
            }),
            Record::Weapon(record) => {
                if let Some(property) = &record.mastery_property {
                    self.mastery_property_refs.push(MasteryPropertyRef {
                        source,
                        weapon: record.id.clone(),
                        property: property.clone(),
                    });
                }
                self.pack.weapons.push(record);
            }
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
            Encoding::Species(encoding) => self.attach_features::<kind::Species>(source, encoding),
            Encoding::Background(encoding) => {
                self.attach_features::<kind::Background>(source, encoding);
            }
            Encoding::Feat(encoding) => self.attach_features::<kind::Feat>(source, encoding),
            Encoding::MasteryProperty(encoding) => {
                self.attach_features::<kind::MasteryProperty>(source, encoding);
            }
        }
    }

    /// Attaches the encoding of a species, background, feat or mastery property to its record.
    fn attach_features<K: FeaturesKind>(&mut self, source: &Source, encoding: FeaturesEncoding) {
        let id = encoding.id.clone();
        if !self.check_id(source, &id, K::NAME) {
            return;
        }
        let Some(found) = self.pack.features_def::<K>(&id) else {
            self.report_id(
                source,
                &id,
                format!("{id} encodes no loaded {} record", K::NAME),
            );
            return;
        };
        let def = &mut self.pack.feature_defs[found.index() as usize];
        if def.encoding.is_some() {
            self.report_id(source, &id, format!("{id} is encoded twice"));
            return;
        }
        def.encoding = Some(encoding);
    }

    /// Reports each weapon whose mastery property is not a mastery property record in the
    /// pack. Runs once every record is loaded, so file order does not matter.
    fn check_mastery_properties(&mut self) {
        for MasteryPropertyRef {
            source,
            weapon,
            property,
        } in std::mem::take(&mut self.mastery_property_refs)
        {
            let known = property.ruleset() == R::PREFIX
                && self
                    .pack
                    .features_def::<kind::MasteryProperty>(&property)
                    .is_some();
            if !known {
                self.report_id(
                    source,
                    &property,
                    format!(
                        "{weapon} has mastery property {property}, which is not a mastery \
                         property in the pack"
                    ),
                );
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
        self.report_quoted(source, &id.to_string(), message);
    }

    /// Reports a problem with `text`, pointing at its first quoted occurrence in the source.
    fn report_quoted(&mut self, source: &Source, text: &str, message: String) {
        self.report_quoted_nth(source, text, 0, message);
    }

    /// Reports a problem with `text`, pointing at its quoted occurrence `nth` (from 0) in
    /// the source, or at its start if there are fewer.
    fn report_quoted_nth(&mut self, source: &Source, text: &str, nth: usize, message: String) {
        let quoted = format!("\"{text}\"");
        let span = source
            .text
            .match_indices(&quoted)
            .nth(nth)
            .map_or((0, 0), |(at, _)| (at, quoted.len()));
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

/// Every `(Ruling, option)` that `pred` can read, in order.
fn ruling_reads<'p>(pred: &'p Pred, reads: &mut Vec<(&'p RulingId, &'p RulingOptionId)>) {
    match pred {
        Pred::Fact(_) => {}
        Pred::Ruling(id, option) => reads.push((id, option)),
        Pred::All(preds) | Pred::Any(preds) => {
            for pred in preds {
                ruling_reads(pred, reads);
            }
        }
        Pred::Not(pred) => ruling_reads(pred, reads),
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
