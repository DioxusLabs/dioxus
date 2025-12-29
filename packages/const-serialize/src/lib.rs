#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::mem::MaybeUninit;

mod const_buffers;
pub use const_buffers::ConstReadBuffer;
mod cbor;
mod const_vec;
mod r#enum;
pub use r#enum::*;
mod r#struct;
pub use r#struct::*;
mod primitive;
pub use primitive::*;
mod list;
pub use list::*;
mod array;
pub use array::*;
mod str;
pub use str::*;

pub use const_serialize_macro::SerializeConst;
pub use const_vec::ConstVec;

use crate::cbor::{
    str_eq, take_array, take_bytes, take_map, take_number, take_str, write_array, write_bytes,
    write_map, write_map_key, write_number,
};

/// The layout for a type. This layout defines a sequence of locations and reversed or not bytes. These bytes will be copied from during serialization and copied into during deserialization.
#[derive(Debug, Copy, Clone)]
pub enum Layout {
    /// An enum layout
    Enum(EnumLayout),
    /// A struct layout
    Struct(StructLayout),
    /// An array layout
    Array(ArrayLayout),
    /// A primitive layout
    Primitive(PrimitiveLayout),
    /// A dynamically sized list layout
    List(ListLayout),
}

impl Layout {
    /// The size of the type in bytes.
    pub const fn size(&self) -> usize {
        match self {
            Layout::Enum(layout) => layout.size,
            Layout::Struct(layout) => layout.size,
            Layout::Array(layout) => layout.len * layout.item_layout.size(),
            Layout::List(layout) => layout.size,
            Layout::Primitive(layout) => layout.size,
        }
    }
}

/// A trait for types that can be serialized and deserialized in const.
///
/// # Safety
/// The layout must accurately describe the memory layout of the type
pub unsafe trait SerializeConst: Sized {
    /// The memory layout of the type. This type must have plain old data; no pointers or references.
    const MEMORY_LAYOUT: Layout;
    /// Assert that the memory layout of the type is the same as the size of the type
    const _ASSERT: () = assert!(Self::MEMORY_LAYOUT.size() == std::mem::size_of::<Self>());
}

/// Serialize a pointer to a type that is stored at the pointer passed in
const unsafe fn serialize_const_ptr(
    ptr: *const (),
    to: ConstVec<u8>,
    layout: &Layout,
) -> ConstVec<u8> {
    match layout {
        Layout::Enum(layout) => serialize_const_enum(ptr, to, layout),
        Layout::Struct(layout) => serialize_const_struct(ptr, to, layout),
        Layout::Array(layout) => serialize_const_array(ptr, to, layout),
        Layout::List(layout) => serialize_const_list(ptr, to, layout),
        Layout::Primitive(layout) => serialize_const_primitive(ptr, to, layout),
    }
}

/// Serialize a type into a buffer
///
/// # Example
///
/// ```rust
/// use const_serialize::{ConstVec, SerializeConst, serialize_const};
///
/// #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
/// struct Struct {
///     a: u32,
///     b: u8,
///     c: u32,
/// }
///
/// let mut buffer = ConstVec::new();
/// buffer = serialize_const(&Struct {
///     a: 0x11111111,
///     b: 0x22,
///     c: 0x33333333,
/// }, buffer);
/// assert_eq!(buffer.as_ref(), &[0xa3, 0x61, 0x61, 0x1a, 0x11, 0x11, 0x11, 0x11, 0x61, 0x62, 0x18, 0x22, 0x61, 0x63, 0x1a, 0x33, 0x33, 0x33, 0x33]);
/// ```
#[must_use = "The data is serialized into the returned buffer"]
pub const fn serialize_const<T: SerializeConst>(data: &T, to: ConstVec<u8>) -> ConstVec<u8> {
    let ptr = data as *const T as *const ();
    // SAFETY: The pointer is valid and the layout is correct
    unsafe { serialize_const_ptr(ptr, to, &T::MEMORY_LAYOUT) }
}

/// Deserialize a type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_ptr<'a>(
    from: &'a [u8],
    layout: &Layout,
    out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    match layout {
        Layout::Enum(layout) => deserialize_const_enum(from, layout, out),
        Layout::Struct(layout) => deserialize_const_struct(from, layout, out),
        Layout::Array(layout) => deserialize_const_array(from, layout, out),
        Layout::List(layout) => deserialize_const_list(from, layout, out),
        Layout::Primitive(layout) => deserialize_const_primitive(from, layout, out),
    }
}

/// Deserialize a type into the output buffer. Accepts `(type, ConstVec<u8>)` as input and returns `Option<(&'a [u8], Instance of type)>`
///
/// # Example
/// ```rust
/// # use const_serialize::{deserialize_const, serialize_const, ConstVec, SerializeConst};
/// #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
/// struct Struct {
///     a: u32,
///     b: u8,
///     c: u32,
///     d: u32,
/// }
///
/// let mut buffer = ConstVec::new();
/// buffer = serialize_const(&Struct {
///     a: 0x11111111,
///     b: 0x22,
///     c: 0x33333333,
///     d: 0x44444444,
/// }, buffer);
/// let buf = buffer.as_ref();
/// assert_eq!(deserialize_const!(Struct, buf).unwrap().1, Struct {
///     a: 0x11111111,
///     b: 0x22,
///     c: 0x33333333,
///     d: 0x44444444,
/// });
/// ```
#[macro_export]
macro_rules! deserialize_const {
    ($type:ty, $buffer:expr) => {
        unsafe {
            const __SIZE: usize = std::mem::size_of::<$type>();
            $crate::deserialize_const_raw::<__SIZE, $type>($buffer)
        }
    };
}

/// Deserialize a buffer into a type. This will return None if the buffer doesn't have enough data to fill the type.
/// # Safety
/// N must be `std::mem::size_of::<T>()`
#[must_use = "The data is deserialized from the input buffer"]
pub const unsafe fn deserialize_const_raw<const N: usize, T: SerializeConst>(
    from: &[u8],
) -> Option<(&[u8], T)> {
    // Create uninitized memory with the size of the type
    let mut out = [MaybeUninit::uninit(); N];
    // Fill in the bytes into the buffer for the type
    let Some(from) = deserialize_const_ptr(from, &T::MEMORY_LAYOUT, &mut out) else {
        return None;
    };
    // Now that the memory is filled in, transmute it into the type
    Some((from, unsafe {
        std::mem::transmute_copy::<[MaybeUninit<u8>; N], T>(&out)
    }))
}

/// Check if the serialized representation of two items are the same
pub const fn serialize_eq<T: SerializeConst>(first: &T, second: &T) -> bool {
    let first_serialized = ConstVec::<u8>::new();
    let first_serialized = serialize_const(first, first_serialized);
    let second_serialized = ConstVec::<u8>::new();
    let second_serialized = serialize_const(second, second_serialized);
    let first_buf = first_serialized.as_ref();
    let second_buf = second_serialized.as_ref();
    if first_buf.len() != second_buf.len() {
        return false;
    }
    let mut i = 0;
    while i < first_buf.len() {
        if first_buf[i] != second_buf[i] {
            return false;
        }
        i += 1;
    }
    true
}
