//! Errors returned by checked buffer operations.

/// Errors that can occur when deserializing objects from a buffer.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum Error {
    /// The buffer does not contain enough bytes.
    #[error(transparent)]
    Truncated(#[from] Truncated),

    /// The buffer contains unexpected trailing bytes.
    #[error(transparent)]
    ExtraneousBytes(#[from] ExtraneousBytes),

    /// The bytes could not be deserialized.
    #[error("deserialization error: {0}")]
    Deserialization(&'static str),
}

/// The buffer does not contain enough bytes.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[error("object truncated (or not fully present)")]
pub struct Truncated;

/// The buffer contains unexpected trailing bytes.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[error("extra bytes at end of object")]
pub struct ExtraneousBytes;
