use crate::*;
use std::mem::MaybeUninit;

/// The layout for a primitive type. The bytes will be reversed if the target is big endian.
#[derive(Debug, Copy, Clone)]
pub struct PrimitiveLayout {
    pub(crate) size: usize,
}

impl PrimitiveLayout {
    /// Create a new primitive layout
    pub const fn new(size: usize) -> Self {
        Self { size }
    }

    /// Read the value from the given pointer
    ///
    /// # Safety
    /// The pointer must be valid for reads of `self.size` bytes.
    pub const unsafe fn read(self, byte_ptr: *const u8) -> u32 {
        let mut value = 0;
        let mut offset = 0;
        while offset < self.size {
            // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
            let byte = if cfg!(target_endian = "big") {
                unsafe {
                    byte_ptr
                        .wrapping_byte_add((self.size - offset - 1) as _)
                        .read()
                }
            } else {
                unsafe { byte_ptr.wrapping_byte_add(offset as _).read() }
            };
            value |= (byte as u32) << (offset * 8);
            offset += 1;
        }
        value
    }

    /// Write the value to the given buffer
    pub const fn write(self, value: u32, out: &mut [MaybeUninit<u8>]) {
        let bytes = value.to_ne_bytes();
        let mut offset = 0;
        while offset < self.size {
            out[offset] = MaybeUninit::new(bytes[offset]);
            offset += 1;
        }
    }
}

macro_rules! impl_serialize_const {
    ($type:ty) => {
        unsafe impl SerializeConst for $type {
            const MEMORY_LAYOUT: Layout = Layout::Primitive(PrimitiveLayout {
                size: std::mem::size_of::<$type>(),
            });
        }
    };
}

impl_serialize_const!(u8);
impl_serialize_const!(u16);
impl_serialize_const!(u32);
impl_serialize_const!(u64);
impl_serialize_const!(i8);
impl_serialize_const!(i16);
impl_serialize_const!(i32);
impl_serialize_const!(i64);
impl_serialize_const!(bool);
impl_serialize_const!(f32);
impl_serialize_const!(f64);

/// Serialize a primitive type that is stored at the pointer passed in
pub(crate) const unsafe fn serialize_const_primitive(
    ptr: *const (),
    to: ConstVec<u8>,
    layout: &PrimitiveLayout,
) -> ConstVec<u8> {
    let ptr = ptr as *const u8;
    let mut offset = 0;
    let mut i64_bytes = [0u8; 8];
    while offset < layout.size {
        // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
        let byte = unsafe {
            if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
                ptr.wrapping_byte_offset((layout.size - offset - 1) as _)
                    .read()
            } else {
                ptr.wrapping_byte_offset(offset as _).read()
            }
        };
        i64_bytes[offset] = byte;
        offset += 1;
    }
    let number = i64::from_ne_bytes(i64_bytes);
    write_number(to, number)
}

/// Deserialize a primitive type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
pub(crate) const fn deserialize_const_primitive<'a>(
    from: &'a [u8],
    layout: &PrimitiveLayout,
    out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    let mut offset = 0;
    let Ok((number, from)) = take_number(from) else {
        return None;
    };
    let bytes = number.to_le_bytes();
    while offset < layout.size {
        // If the bytes are reversed, walk backwards from the end of the number when filling in bytes
        let byte = bytes[offset];
        if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
            out[layout.size - offset - 1] = MaybeUninit::new(byte);
        } else {
            out[offset] = MaybeUninit::new(byte);
        }
        offset += 1;
    }
    Some(from)
}
