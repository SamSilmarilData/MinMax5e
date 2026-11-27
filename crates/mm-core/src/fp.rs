//! The 128-bit fingerprint (ADR-0052).

use std::fmt;

use serde::{Serialize, Serializer};

/// A fingerprint: blake3 truncated to 128 bits. Every cache key and stage key is one or a
/// tuple of them (architecture §1). The same input bytes give the same `Fp` on every target.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fp([u8; 16]);

impl fmt::Display for Fp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for Fp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fp({self})")
    }
}

impl Serialize for Fp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Builds an [`Fp`] from a sequence of values. Each write is self-delimiting (strings are
/// length-prefixed, integers fixed-width little-endian), so different sequences of writes
/// do not collide by concatenation.
#[derive(Debug, Clone)]
pub struct FpHasher(blake3::Hasher);

impl FpHasher {
    /// A hasher for one kind of key. `domain` separates kinds, so a `ShapeKey` and a
    /// `ParamsFp` over equal bytes still differ.
    pub fn new(domain: &str) -> FpHasher {
        let mut hasher = FpHasher(blake3::Hasher::new());
        hasher.str(domain);
        hasher
    }

    pub fn u64(&mut self, value: u64) -> &mut Self {
        self.0.update(&value.to_le_bytes());
        self
    }

    pub fn i64(&mut self, value: i64) -> &mut Self {
        self.0.update(&value.to_le_bytes());
        self
    }

    pub fn str(&mut self, value: &str) -> &mut Self {
        self.u64(value.len() as u64);
        self.0.update(value.as_bytes());
        self
    }

    pub fn finish(&self) -> Fp {
        let mut out = [0; 16];
        out.copy_from_slice(&self.0.finalize().as_bytes()[..16]);
        Fp(out)
    }
}
