//! Rulings (ADR-0006): named, cited choices between readings of ambiguous rules text, and
//! the set of options a user has chosen.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Citation;

/// A Ruling's stable id: `R-` then uppercase letters, digits and hyphens, for example
/// `R-TST-HEW-BONUS`.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RulingId(Box<str>);

/// Why a string is not a Ruling id.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a Ruling id (`R-` then uppercase letters, digits and hyphens)")]
pub struct RulingIdError(pub String);

impl RulingId {
    /// Parses `R-` followed by uppercase letters, digits and hyphens, not ending in one.
    ///
    /// # Errors
    /// If `text` is not of that form.
    pub fn parse(text: &str) -> Result<RulingId, RulingIdError> {
        let valid = text.strip_prefix("R-").is_some_and(|rest| {
            !rest.is_empty()
                && !rest.ends_with('-')
                && rest
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        });
        if valid {
            Ok(RulingId(text.into()))
        } else {
            Err(RulingIdError(text.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RulingId {
    type Error = RulingIdError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        RulingId::parse(&text)
    }
}

impl From<RulingId> for String {
    fn from(id: RulingId) -> String {
        id.0.into()
    }
}

impl fmt::Display for RulingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for RulingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RulingId({})", self.0)
    }
}

/// A Ruling as a pack declares it: one file under a pack's `rulings/` directory.
///
/// An option's meaning never changes once results have recorded it: a changed reading is
/// a new option id. So an `(id, option)` pair names one reading for good, and is the
/// Ruling's version (ADR-0006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ruling {
    pub id: RulingId,
    /// The question the Ruling answers, in our words.
    pub question: String,
    /// The readings, each with a stable id; at least two.
    pub options: Vec<RulingOption>,
    /// The id of the option used when a Ruling Set does not choose one.
    pub default: String,
    /// Why the default is the default.
    pub reasoning: String,
    /// Where the readings come from (ADR-0007); at least one.
    pub citations: Vec<Citation>,
}

/// One reading a Ruling offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulingOption {
    /// Lowercase letters, digits and hyphens.
    pub id: String,
    /// What this reading says, in our words.
    pub reading: String,
}

impl Ruling {
    /// Whether `option` is one of this Ruling's option ids.
    pub fn has_option(&self, option: &str) -> bool {
        self.options.iter().any(|o| o.id == option)
    }

    /// The option ids, in declaration order, joined with `, `.
    pub fn option_list(&self) -> String {
        let ids: Vec<&str> = self.options.iter().map(|o| o.id.as_str()).collect();
        ids.join(", ")
    }
}

/// The options a user has chosen for some Rulings: the rest take their defaults.
///
/// As a file (`mm-cli evaluate --rulings`), it is a RON map from Ruling id to option id:
///
/// ```ron
/// {
///     "R-TST-HEW-BONUS": "bonus-action-too",
/// }
/// ```
///
/// Whether each Ruling and option exists is checked against the pack when a Build compiles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RulingSet {
    pub chosen: BTreeMap<RulingId, String>,
}
