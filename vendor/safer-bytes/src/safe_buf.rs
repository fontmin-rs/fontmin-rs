//! Checked extension methods for [`bytes::Buf`].

use crate::{error, FromBuf};
use bytes::{Buf, Bytes};

macro_rules! get_primitive_checked {
    ($checked:ident, $unchecked:ident, $type:ty, $width:literal) => {
        fn $checked(&mut self) -> std::result::Result<$type, error::Truncated> {
            if self.remaining() >= $width {
                Ok(self.$unchecked())
            } else {
                Err(error::Truncated)
            }
        }
    };
}

/// Checked extension methods for [`bytes::Buf`].
pub trait SafeBuf: Buf {
    /// Copies `len` bytes after checking that they are available.
    fn try_copy_to_bytes(&mut self, len: usize) -> std::result::Result<Bytes, error::Truncated> {
        if self.remaining() < len {
            Err(error::Truncated)
        } else {
            Ok(self.copy_to_bytes(len))
        }
    }

    /// Copies bytes into `dst` after checking that they are available.
    fn try_copy_to_slice(&mut self, dst: &mut [u8]) -> std::result::Result<(), error::Truncated> {
        if self.remaining() < dst.len() {
            Err(error::Truncated)
        } else {
            self.copy_to_slice(dst);
            Ok(())
        }
    }

    /// Reads a custom object from this buffer.
    fn extract<T>(&mut self) -> crate::Result<T>
    where
        T: FromBuf,
    {
        T::from_buf(self)
    }

    /// Verifies that the buffer has no trailing bytes.
    fn should_be_exhausted(&self) -> std::result::Result<(), error::ExtraneousBytes> {
        if self.has_remaining() {
            Err(error::ExtraneousBytes)
        } else {
            Ok(())
        }
    }

    get_primitive_checked!(try_get_u8, get_u8, u8, 1);
    get_primitive_checked!(try_get_i8, get_i8, i8, 1);
    get_primitive_checked!(try_get_u16, get_u16, u16, 2);
    get_primitive_checked!(try_get_i16, get_i16, i16, 2);
    get_primitive_checked!(try_get_u32, get_u32, u32, 4);
    get_primitive_checked!(try_get_i32, get_i32, i32, 4);
    get_primitive_checked!(try_get_u64, get_u64, u64, 8);
    get_primitive_checked!(try_get_i64, get_i64, i64, 8);
    get_primitive_checked!(try_get_u128, get_u128, u128, 16);
    get_primitive_checked!(try_get_i128, get_i128, i128, 16);

    get_primitive_checked!(try_get_u16_le, get_u16_le, u16, 2);
    get_primitive_checked!(try_get_i16_le, get_i16_le, i16, 2);
    get_primitive_checked!(try_get_u32_le, get_u32_le, u32, 4);
    get_primitive_checked!(try_get_i32_le, get_i32_le, i32, 4);
    get_primitive_checked!(try_get_u64_le, get_u64_le, u64, 8);
    get_primitive_checked!(try_get_i64_le, get_i64_le, i64, 8);
    get_primitive_checked!(try_get_u128_le, get_u128_le, u128, 16);
    get_primitive_checked!(try_get_i128_le, get_i128_le, i128, 16);
}

impl<T> SafeBuf for T where T: Buf {}
