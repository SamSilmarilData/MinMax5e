//! Where an encoded feature's mechanics come from, and how far that source is trusted (ADR-0007).

use serde::{Deserialize, Serialize};

/// The text an encoded feature was written from, and its tier in the order of trust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub tier: SourceTier,
    /// Where in that source, e.g. a section or page. Never the rules text itself
    /// (ADR-0028).
    pub reference: String,
}

/// The tiers of ADR-0007's order of trust that an encoded feature can cite, most trusted first.
/// Sage Advice and community consensus inform Ruling options, not encodings, so they
/// are not tiers here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceTier {
    /// Official errata.
    Errata,
    /// The SRD 5.2.x text.
    Srd,
    /// Book text read from the local corpus (ADR-0027): a community transcription, so a
    /// lower tier than the SRD.
    Corpus,
    /// Invented for tests and fixtures; describes no published rule (ADR-0028).
    Synthetic,
}
