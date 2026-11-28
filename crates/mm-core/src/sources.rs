//! Named RON texts: the input to loading a Rules Pack.

/// The texts a pack is loaded from. Each keeps its name, so diagnostics can point into it.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    pub files: Vec<Source>,
}

/// One named text.
#[derive(Debug, Clone)]
pub struct Source {
    /// Where the text came from, e.g. a path, for diagnostics.
    pub name: String,
    pub role: SourceRole,
    pub text: String,
}

/// What a text holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRole {
    /// A list of resolved records ([`records::Record`](crate::records::Record)).
    Records,
    /// One encoding in the Effect DSL ([`effect::Encoding`](crate::effect::Encoding)).
    Encoding,
    /// One Ruling's declaration ([`Ruling`](crate::Ruling)).
    Ruling,
}

impl Sources {
    /// These sources followed by `more`'s.
    #[must_use]
    pub fn chain(mut self, more: Sources) -> Sources {
        self.files.extend(more.files);
        self
    }
}
