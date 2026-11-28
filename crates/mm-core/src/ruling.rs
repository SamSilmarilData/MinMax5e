//! Rulings (ADR-0006): named, cited choices between readings of ambiguous rules text, and
//! the set of options a user has chosen.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Citation;

/// Declares a validated string id: its parser, serde through `String`, `Display` and
/// `Debug`.
macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident, $error:ident, $what:literal, $form:literal, $valid:expr) => {
        $(#[$doc])*
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Box<str>);

        #[doc = concat!("Why a string is not ", $what, ".")]
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        #[error("`{text}` is not {what} ({form})", text = .0, what = $what, form = $form)]
        pub struct $error(pub String);

        impl $name {
            #[doc = concat!("Parses ", $what, ": ", $form, ".")]
            ///
            /// # Errors
            /// If `text` is not of that form.
            pub fn parse(text: &str) -> Result<$name, $error> {
                let valid: fn(&str) -> bool = $valid;
                if valid(text) {
                    Ok($name(text.into()))
                } else {
                    Err($error(text.to_owned()))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = $error;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                $name::parse(&text)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> String {
                id.0.into()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

string_id!(
    /// A Ruling's stable id: `R-` then uppercase letters, digits and hyphens, not ending in
    /// a hyphen, for example `R-TST-HEW-BONUS`.
    RulingId,
    RulingIdError,
    "a Ruling id",
    "`R-` then uppercase letters, digits and hyphens, not ending in a hyphen",
    |text| {
        text.strip_prefix("R-").is_some_and(|rest| {
            !rest.is_empty()
                && !rest.ends_with('-')
                && rest
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        })
    }
);

string_id!(
    /// One option's stable id within its Ruling: lowercase letters, digits and hyphens,
    /// starting with a letter and not ending in a hyphen, for example `bonus-action-too`.
    RulingOptionId,
    RulingOptionIdError,
    "a Ruling option id",
    "lowercase letters, digits and hyphens, starting with a letter and not ending in a hyphen",
    |text| {
        text.starts_with(|c: char| c.is_ascii_lowercase())
            && !text.ends_with('-')
            && text
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }
);

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
    pub default: RulingOptionId,
    /// Why the default is the default.
    pub reasoning: String,
    /// Where the readings come from (ADR-0007); at least one.
    pub citations: Vec<Citation>,
}

/// One reading a Ruling offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulingOption {
    pub id: RulingOptionId,
    /// What this reading says, in our words.
    pub reading: String,
}

impl Ruling {
    /// Whether `option` is one of this Ruling's options.
    ///
    /// # Errors
    /// If it is not, naming the options there are.
    pub fn check_option(&self, option: &RulingOptionId) -> Result<(), RulingError> {
        if self.options.iter().any(|o| &o.id == option) {
            return Ok(());
        }
        Err(RulingError::UnknownOption {
            ruling: self.id.clone(),
            option: option.clone(),
            options: self.options.iter().map(|o| o.id.clone()).collect(),
        })
    }
}

/// A Ruling, or an option of one, named but not declared.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RulingError {
    #[error("there is no Ruling `{0}` in the pack")]
    Unknown(RulingId),
    #[error(
        "Ruling `{ruling}` has no option `{option}`; its options are {}",
        options.iter().map(RulingOptionId::as_str).collect::<Vec<_>>().join(", ")
    )]
    UnknownOption {
        ruling: RulingId,
        option: RulingOptionId,
        /// The Ruling's option ids, in declaration order.
        options: Vec<RulingOptionId>,
    },
}

/// The options a user has chosen for some Rulings: the rest take their defaults.
///
/// The same map also records the Rulings a kit or result read, each with the option it
/// had (defaults included), so a result's Rulings can be passed back as a Ruling Set.
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
    pub chosen: BTreeMap<RulingId, RulingOptionId>,
}
