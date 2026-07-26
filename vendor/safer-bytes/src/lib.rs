//! A safe, non-panicking wrapper around [`bytes::Buf`].

use bytes::Buf;
pub use bytes::{BufMut, Bytes, BytesMut};

pub mod error;
mod safe_buf;

/// Unchecked buffer reading methods.
pub mod unchecked {
    pub use bytes::Buf;
}

#[doc(inline)]
pub use error::Error;
pub use safe_buf::SafeBuf;

/// Type alias for the return type of fallible functions in this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A value that can be constructed by reading bytes from a buffer.
pub trait FromBuf: Sized {
    /// Reads an instance of `Self` from a buffer.
    ///
    /// # Errors
    ///
    /// Returns an error when the buffer is truncated or cannot be parsed.
    fn from_buf<B>(buffer: B) -> Result<Self>
    where
        B: Buf;
}
