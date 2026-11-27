//! Content IDs (ADR-0008) and the typed handles a Rules Pack hands out.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Ruleset;
use crate::kind::Kind;

/// The identity of one piece of content: `<ruleset>:<kind>/<slug>|<source>`, for example
/// `2024:class/test-warrior|tst` (ADR-0008). Composite paths (`<kind>/<a>/<b>`) name
/// subclasses and features.
///
/// Parsing checks the form only. Whether the ruleset is the right one, and whether the ID
/// names anything, is checked when a pack loads.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentId {
    text: Box<str>,
    /// Byte offsets of the `:`, the first `/` and the `|`.
    colon: usize,
    slash: usize,
    bar: usize,
}

/// Why a string is not a Content ID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{text}` is not a Content ID (`<ruleset>:<kind>/<slug>|<source>`): {reason}")]
pub struct ContentIdError {
    pub text: String,
    pub reason: &'static str,
}

impl ContentId {
    /// Parses `<ruleset>:<kind>/<slug>[/<slug>…]|<source>`. The ruleset and source are
    /// lowercase letters and digits; the kind and slugs are lowercase letters, digits and
    /// hyphens, the kind starting with a letter.
    ///
    /// # Errors
    /// If `text` is not of that form.
    pub fn parse(text: &str) -> Result<ContentId, ContentIdError> {
        let fail = |reason| ContentIdError {
            text: text.to_owned(),
            reason,
        };
        let colon = text
            .find(':')
            .ok_or_else(|| fail("no `:` after the ruleset"))?;
        let bar = text.rfind('|').ok_or_else(|| fail("no `|source`"))?;
        let slash = text[colon..bar]
            .find('/')
            .map(|i| colon + i)
            .ok_or_else(|| fail("no `/` after the kind"))?;

        let alnum = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        };
        let slug = |s: &str| {
            s.bytes()
                .next()
                .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        };
        if !alnum(&text[..colon]) {
            return Err(fail("the ruleset is not lowercase letters and digits"));
        }
        let kind = &text[colon + 1..slash];
        if !(slug(kind) && kind.starts_with(|c: char| c.is_ascii_lowercase())) {
            return Err(fail("the kind is not a lowercase word"));
        }
        if !text[slash + 1..bar].split('/').all(slug) {
            return Err(fail("a slug is not lowercase letters, digits and hyphens"));
        }
        if !alnum(&text[bar + 1..]) {
            return Err(fail("the source is not lowercase letters and digits"));
        }
        Ok(ContentId {
            text: text.into(),
            colon,
            slash,
            bar,
        })
    }

    /// The Ruleset prefix, e.g. `2024`.
    pub fn ruleset(&self) -> &str {
        &self.text[..self.colon]
    }

    /// The kind, e.g. `class`.
    pub fn kind(&self) -> &str {
        &self.text[self.colon + 1..self.slash]
    }

    /// The slug path after the kind, e.g. `test-warrior`.
    pub fn path(&self) -> &str {
        &self.text[self.slash + 1..self.bar]
    }

    /// The source, e.g. `tst`.
    pub fn source(&self) -> &str {
        &self.text[self.bar + 1..]
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &*self.text)
    }
}

impl Serialize for ContentId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text)
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        ContentId::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// A handle to one item of kind `K` in a Rules Pack of Ruleset `R`: a dense index, valid
/// for the pack that issued it. `mm-load` issues them when it resolves Content IDs.
pub struct Id<R, K> {
    index: u32,
    marker: PhantomData<fn() -> (R, K)>,
}

impl<R: Ruleset, K: Kind> Id<R, K> {
    /// The handle for item `index` of kind `K`. Only the pack that owns the items should
    /// make one.
    pub fn from_index(index: u32) -> Id<R, K> {
        Id {
            index,
            marker: PhantomData,
        }
    }

    pub fn index(self) -> u32 {
        self.index
    }
}

// Derives would bound `R` and `K` too.
impl<R, K> Clone for Id<R, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R, K> Copy for Id<R, K> {}
impl<R, K> PartialEq for Id<R, K> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<R, K> Eq for Id<R, K> {}
impl<R, K> Hash for Id<R, K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
impl<R, K: Kind> fmt::Debug for Id<R, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", K::NAME, self.index)
    }
}
