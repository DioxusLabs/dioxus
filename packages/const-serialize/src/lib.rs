#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{char, mem::MaybeUninit};

mod const_buffers;
mod const_vec;

pub use const_buffers::ConstReadBuffer;
pub use const_serialize_macro::SerializeConst;
pub use const_vec::ConstVec;

/// Plain old data for a field. Stores the offset of the field in the struct and the layout of the field.
#[derive(Debug, Copy, Clone)]
pub struct StructFieldLayout {
    offset: usize,
    layout: Layout,
}

impl StructFieldLayout {
    /// Create a new struct field layout
    pub const fn new(offset: usize, layout: Layout) -> Self {
        Self { offset, layout }
    }
}

/// Layout for a struct. The struct layout is just a list of fields with offsets
#[derive(Debug, Copy, Clone)]
pub struct StructLayout {
    size: usize,
    data: &'static [StructFieldLayout],
}

impl StructLayout {
    /// Create a new struct layout
    pub const fn new(size: usize, data: &'static [StructFieldLayout]) -> Self {
        Self { size, data }
    }
}

/// The layout for an enum. The enum layout is just a discriminate size and a tag layout.
#[derive(Debug, Copy, Clone)]
pub struct EnumLayout {
    size: usize,
    discriminant: PrimitiveLayout,
    variants_offset: usize,
    variants: &'static [EnumVariant],
}

impl EnumLayout {
    /// Create a new enum layout
    pub const fn new(
        size: usize,
        discriminant: PrimitiveLayout,
        variants: &'static [EnumVariant],
    ) -> Self {
        let mut max_align = 1;
        let mut i = 0;
        while i < variants.len() {
            let EnumVariant { align, .. } = &variants[i];
            if *align > max_align {
                max_align = *align;
            }
            i += 1;
        }

        let variants_offset_raw = discriminant.size;
        let padding = (max_align - (variants_offset_raw % max_align)) % max_align;
        let variants_offset = variants_offset_raw + padding;

        assert!(variants_offset % max_align == 0);

        Self {
            size,
            discriminant,
            variants_offset,
            variants,
        }
    }
}

/// The layout for an enum variant. The enum variant layout is just a struct layout with a tag and alignment.
#[derive(Debug, Copy, Clone)]
pub struct EnumVariant {
    // Note: tags may not be sequential
    tag: u32,
    data: StructLayout,
    align: usize,
}

impl EnumVariant {
    /// Create a new enum variant layout
    pub const fn new(tag: u32, data: StructLayout, align: usize) -> Self {
        Self { tag, data, align }
    }
}

/// The layout for a constant sized array. The array layout is just a length and an item layout.
#[derive(Debug, Copy, Clone)]
pub struct ListLayout {
    len: usize,
    item_layout: &'static Layout,
}

impl ListLayout {
    /// Create a new list layout
    pub const fn new(len: usize, item_layout: &'static Layout) -> Self {
        Self { len, item_layout }
    }
}

/// The layout for a primitive type. The bytes will be reversed if the target is big endian.
#[derive(Debug, Copy, Clone)]
pub struct PrimitiveLayout {
    size: usize,
}

impl PrimitiveLayout {
    /// Create a new primitive layout
    pub const fn new(size: usize) -> Self {
        Self { size }
    }
}

/// The layout for a type. This layout defines a sequence of locations and reversed or not bytes. These bytes will be copied from during serialization and copied into during deserialization.
#[derive(Debug, Copy, Clone)]
pub enum Layout {
    /// An enum layout
    Enum(EnumLayout),
    /// A struct layout
    Struct(StructLayout),
    /// A list layout
    List(ListLayout),
    /// A primitive layout
    Primitive(PrimitiveLayout),
}

impl Layout {
    /// The size of the type in bytes.
    const fn size(&self) -> usize {
        match self {
            Layout::Enum(layout) => layout.size,
            Layout::Struct(layout) => layout.size,
            Layout::List(layout) => layout.len * layout.item_layout.size(),
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

unsafe impl<const N: usize, T: SerializeConst> SerializeConst for [T; N] {
    const MEMORY_LAYOUT: Layout = Layout::List(ListLayout {
        len: N,
        item_layout: &T::MEMORY_LAYOUT,
    });
}

macro_rules! impl_serialize_const_tuple {
    ($($generic:ident: $generic_number:expr),*) => {
        impl_serialize_const_tuple!(@impl ($($generic,)*) = $($generic: $generic_number),*);
    };
    (@impl $inner:ty = $($generic:ident: $generic_number:expr),*) => {
        unsafe impl<$($generic: SerializeConst),*> SerializeConst for ($($generic,)*) {
            const MEMORY_LAYOUT: Layout = {
                Layout::Struct(StructLayout {
                    size: std::mem::size_of::<($($generic,)*)>(),
                    data: &[
                        $(
                            StructFieldLayout::new(std::mem::offset_of!($inner, $generic_number), $generic::MEMORY_LAYOUT),
                        )*
                    ],
                })
            };
        }
    };
}

impl_serialize_const_tuple!(T1: 0);
impl_serialize_const_tuple!(T1: 0, T2: 1);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4, T6: 5);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4, T6: 5, T7: 6);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4, T6: 5, T7: 6, T8: 7);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4, T6: 5, T7: 6, T8: 7, T9: 8);
impl_serialize_const_tuple!(T1: 0, T2: 1, T3: 2, T4: 3, T5: 4, T6: 5, T7: 6, T8: 7, T9: 8, T10: 9);

const MAX_STR_SIZE: usize = 256;

/// A string that is stored in a constant sized buffer that can be serialized and deserialized at compile time
#[derive(PartialEq, PartialOrd, Clone, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConstStr {
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    bytes: [u8; MAX_STR_SIZE],
    len: u32,
}

#[cfg(feature = "serde")]
mod serde_bytes {
    use serde::{Deserialize, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; super::MAX_STR_SIZE], D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Failed to convert bytes to a fixed size array"))
    }
}

unsafe impl SerializeConst for ConstStr {
    const MEMORY_LAYOUT: Layout = Layout::Struct(StructLayout {
        size: std::mem::size_of::<Self>(),
        data: &[
            StructFieldLayout::new(
                std::mem::offset_of!(Self, bytes),
                Layout::List(ListLayout {
                    len: MAX_STR_SIZE,
                    item_layout: &Layout::Primitive(PrimitiveLayout {
                        size: std::mem::size_of::<u8>(),
                    }),
                }),
            ),
            StructFieldLayout::new(
                std::mem::offset_of!(Self, len),
                Layout::Primitive(PrimitiveLayout {
                    size: std::mem::size_of::<u32>(),
                }),
            ),
        ],
    });
}

impl ConstStr {
    /// Create a new constant string
    pub const fn new(s: &str) -> Self {
        let str_bytes = s.as_bytes();
        let mut bytes = [0; MAX_STR_SIZE];
        let mut i = 0;
        while i < str_bytes.len() {
            bytes[i] = str_bytes[i];
            i += 1;
        }
        Self {
            bytes,
            len: str_bytes.len() as u32,
        }
    }

    /// Get a reference to the string
    pub const fn as_str(&self) -> &str {
        let str_bytes = self.bytes.split_at(self.len as usize).0;
        match std::str::from_utf8(str_bytes) {
            Ok(s) => s,
            Err(_) => panic!(
                "Invalid utf8; ConstStr should only ever be constructed from valid utf8 strings"
            ),
        }
    }

    /// Get the length of the string
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if the string is empty
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push a character onto the string
    pub const fn push(self, byte: char) -> Self {
        assert!(byte.is_ascii(), "Only ASCII bytes are supported");
        let (bytes, len) = char_to_bytes(byte);
        let (str, _) = bytes.split_at(len);
        let Ok(str) = std::str::from_utf8(str) else {
            panic!("Invalid utf8; char_to_bytes should always return valid utf8 bytes")
        };
        self.push_str(str)
    }

    /// Push a str onto the string
    pub const fn push_str(self, str: &str) -> Self {
        let Self { mut bytes, len } = self;
        assert!(
            str.len() + len as usize <= MAX_STR_SIZE,
            "String is too long"
        );
        let str_bytes = str.as_bytes();
        let new_len = len as usize + str_bytes.len();
        let mut i = 0;
        while i < str_bytes.len() {
            bytes[len as usize + i] = str_bytes[i];
            i += 1;
        }
        Self {
            bytes,
            len: new_len as u32,
        }
    }

    /// Split the string at a byte index. The byte index must be a char boundary
    pub const fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.bytes.split_at(index);
        let left = match std::str::from_utf8(left) {
            Ok(s) => s,
            Err(_) => {
                panic!("Invalid utf8; you cannot split at a byte that is not a char boundary")
            }
        };
        let right = match std::str::from_utf8(right) {
            Ok(s) => s,
            Err(_) => {
                panic!("Invalid utf8; you cannot split at a byte that is not a char boundary")
            }
        };
        (Self::new(left), Self::new(right))
    }

    /// Split the string at the last occurrence of a character
    pub const fn rsplit_once(&self, char: char) -> Option<(Self, Self)> {
        let str = self.as_str();
        let mut index = str.len() - 1;
        // First find the bytes we are searching for
        let (char_bytes, len) = char_to_bytes(char);
        let (char_bytes, _) = char_bytes.split_at(len);
        let bytes = str.as_bytes();

        // Then walk backwards from the end of the string
        loop {
            let byte = bytes[index];
            // Look for char boundaries in the string and check if the bytes match
            if let Some(char_boundary_len) = utf8_char_boundary_to_char_len(byte) {
                // Split up the string into three sections: [before_char, in_char, after_char]
                let (before_char, after_index) = bytes.split_at(index);
                let (in_char, after_char) = after_index.split_at(char_boundary_len as usize);
                if in_char.len() != char_boundary_len as usize {
                    panic!("in_char.len() should always be equal to char_boundary_len as usize")
                }
                // Check if the bytes for the current char and the target char match
                let mut in_char_eq = true;
                let mut i = 0;
                let min_len = if in_char.len() < char_bytes.len() {
                    in_char.len()
                } else {
                    char_bytes.len()
                };
                while i < min_len {
                    in_char_eq &= in_char[i] == char_bytes[i];
                    i += 1;
                }
                // If they do, convert the bytes to strings and return the split strings
                if in_char_eq {
                    let Ok(before_char_str) = std::str::from_utf8(before_char) else {
                        panic!("Invalid utf8; utf8_char_boundary_to_char_len should only return Some when the byte is a character boundary")
                    };
                    let Ok(after_char_str) = std::str::from_utf8(after_char) else {
                        panic!("Invalid utf8; utf8_char_boundary_to_char_len should only return Some when the byte is a character boundary")
                    };
                    return Some((Self::new(before_char_str), Self::new(after_char_str)));
                }
            }
            match index.checked_sub(1) {
                Some(new_index) => index = new_index,
                None => return None,
            }
        }
    }

    /// Split the string at the first occurrence of a character
    pub const fn split_once(&self, char: char) -> Option<(Self, Self)> {
        let str = self.as_str();
        let mut index = 0;
        // First find the bytes we are searching for
        let (char_bytes, len) = char_to_bytes(char);
        let (char_bytes, _) = char_bytes.split_at(len);
        let bytes = str.as_bytes();

        // Then walk forwards from the start of the string
        while index < bytes.len() {
            let byte = bytes[index];
            // Look for char boundaries in the string and check if the bytes match
            if let Some(char_boundary_len) = utf8_char_boundary_to_char_len(byte) {
                // Split up the string into three sections: [before_char, in_char, after_char]
                let (before_char, after_index) = bytes.split_at(index);
                let (in_char, after_char) = after_index.split_at(char_boundary_len as usize);
                if in_char.len() != char_boundary_len as usize {
                    panic!("in_char.len() should always be equal to char_boundary_len as usize")
                }
                // Check if the bytes for the current char and the target char match
                let mut in_char_eq = true;
                let mut i = 0;
                let min_len = if in_char.len() < char_bytes.len() {
                    in_char.len()
                } else {
                    char_bytes.len()
                };
                while i < min_len {
                    in_char_eq &= in_char[i] == char_bytes[i];
                    i += 1;
                }
                // If they do, convert the bytes to strings and return the split strings
                if in_char_eq {
                    let Ok(before_char_str) = std::str::from_utf8(before_char) else {
                        panic!("Invalid utf8; utf8_char_boundary_to_char_len should only return Some when the byte is a character boundary")
                    };
                    let Ok(after_char_str) = std::str::from_utf8(after_char) else {
                        panic!("Invalid utf8; utf8_char_boundary_to_char_len should only return Some when the byte is a character boundary")
                    };
                    return Some((Self::new(before_char_str), Self::new(after_char_str)));
                }
            }
            index += 1
        }
        None
    }
}

impl std::fmt::Debug for ConstStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

#[test]
fn test_rsplit_once() {
    let str = ConstStr::new("hello world");
    assert_eq!(
        str.rsplit_once(' '),
        Some((ConstStr::new("hello"), ConstStr::new("world")))
    );

    let unicode_str = ConstStr::new("hi😀hello😀world😀world");
    assert_eq!(
        unicode_str.rsplit_once('😀'),
        Some((ConstStr::new("hi😀hello😀world"), ConstStr::new("world")))
    );
    assert_eq!(unicode_str.rsplit_once('❌'), None);

    for _ in 0..100 {
        let random_str: String = (0..rand::random::<u8>() % 50)
            .map(|_| rand::random::<char>())
            .collect();
        let konst = ConstStr::new(&random_str);
        let mut seen_chars = std::collections::HashSet::new();
        for char in random_str.chars().rev() {
            let (char_bytes, len) = char_to_bytes(char);
            let char_bytes = &char_bytes[..len];
            assert_eq!(char_bytes, char.to_string().as_bytes());
            if seen_chars.contains(&char) {
                continue;
            }
            seen_chars.insert(char);
            let (correct_left, correct_right) = random_str.rsplit_once(char).unwrap();
            let (left, right) = konst.rsplit_once(char).unwrap();
            println!("splitting {random_str:?} at {char:?}");
            assert_eq!(left.as_str(), correct_left);
            assert_eq!(right.as_str(), correct_right);
        }
    }
}

const CONTINUED_CHAR_MASK: u8 = 0b10000000;
const BYTE_CHAR_BOUNDARIES: [u8; 4] = [0b00000000, 0b11000000, 0b11100000, 0b11110000];

// Const version of https://doc.rust-lang.org/src/core/char/methods.rs.html#1765-1797
const fn char_to_bytes(char: char) -> ([u8; 4], usize) {
    let code = char as u32;
    let len = char.len_utf8();
    let mut bytes = [0; 4];
    match len {
        1 => {
            bytes[0] = code as u8;
        }
        2 => {
            bytes[0] = (code >> 6 & 0x1F) as u8 | BYTE_CHAR_BOUNDARIES[1];
            bytes[1] = (code & 0x3F) as u8 | CONTINUED_CHAR_MASK;
        }
        3 => {
            bytes[0] = (code >> 12 & 0x0F) as u8 | BYTE_CHAR_BOUNDARIES[2];
            bytes[1] = (code >> 6 & 0x3F) as u8 | CONTINUED_CHAR_MASK;
            bytes[2] = (code & 0x3F) as u8 | CONTINUED_CHAR_MASK;
        }
        4 => {
            bytes[0] = (code >> 18 & 0x07) as u8 | BYTE_CHAR_BOUNDARIES[3];
            bytes[1] = (code >> 12 & 0x3F) as u8 | CONTINUED_CHAR_MASK;
            bytes[2] = (code >> 6 & 0x3F) as u8 | CONTINUED_CHAR_MASK;
            bytes[3] = (code & 0x3F) as u8 | CONTINUED_CHAR_MASK;
        }
        _ => panic!(
            "encode_utf8: need more than 4 bytes to encode the unicode character, but the buffer has 4 bytes"
        ),
    };
    (bytes, len)
}

#[test]
fn fuzz_char_to_bytes() {
    use std::char;
    for _ in 0..100 {
        let char = rand::random::<char>();
        let (bytes, len) = char_to_bytes(char);
        let str = std::str::from_utf8(&bytes[..len]).unwrap();
        assert_eq!(char.to_string(), str);
    }
}

const fn utf8_char_boundary_to_char_len(byte: u8) -> Option<u8> {
    match byte {
        0b00000000..=0b01111111 => Some(1),
        0b11000000..=0b11011111 => Some(2),
        0b11100000..=0b11101111 => Some(3),
        0b11110000..=0b11111111 => Some(4),
        _ => None,
    }
}

#[test]
fn fuzz_utf8_byte_to_char_len() {
    for _ in 0..100 {
        let random_string: String = (0..rand::random::<u8>())
            .map(|_| rand::random::<char>())
            .collect();
        let bytes = random_string.as_bytes();
        let chars: std::collections::HashMap<_, _> = random_string.char_indices().collect();
        for (i, byte) in bytes.iter().enumerate() {
            match utf8_char_boundary_to_char_len(*byte) {
                Some(char_len) => {
                    let char = chars
                        .get(&i)
                        .unwrap_or_else(|| panic!("{byte:b} is not a character boundary"));
                    assert_eq!(char.len_utf8(), char_len as usize);
                }
                None => {
                    assert!(!chars.contains_key(&i), "{byte:b} is a character boundary");
                }
            }
        }
    }
}

/// Serialize a struct that is stored at the pointer passed in
const fn serialize_const_struct(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &StructLayout,
) -> ConstVec<u8> {
    let mut i = 0;
    while i < layout.data.len() {
        // Serialize the field at the offset pointer in the struct
        let StructFieldLayout { offset, layout } = &layout.data[i];
        let field = ptr.wrapping_byte_add(*offset as _);
        to = serialize_const_ptr(field, to, layout);
        i += 1;
    }
    to
}

/// Serialize an enum that is stored at the pointer passed in
const fn serialize_const_enum(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &EnumLayout,
) -> ConstVec<u8> {
    let mut discriminant = 0;

    let byte_ptr = ptr as *const u8;
    let mut offset = 0;
    while offset < layout.discriminant.size {
        // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
        let byte = if cfg!(target_endian = "big") {
            unsafe {
                byte_ptr
                    .wrapping_byte_add((layout.discriminant.size - offset - 1) as _)
                    .read()
            }
        } else {
            unsafe { byte_ptr.wrapping_byte_add(offset as _).read() }
        };
        to = to.push(byte);
        discriminant |= (byte as u32) << (offset * 8);
        offset += 1;
    }

    let mut i = 0;
    while i < layout.variants.len() {
        // If the variant is the discriminated one, serialize it
        let EnumVariant { tag, data, .. } = &layout.variants[i];
        if discriminant == *tag {
            let data_ptr = ptr.wrapping_byte_offset(layout.variants_offset as _);
            to = serialize_const_struct(data_ptr, to, data);
            break;
        }
        i += 1;
    }
    to
}

/// Serialize a primitive type that is stored at the pointer passed in
const fn serialize_const_primitive(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &PrimitiveLayout,
) -> ConstVec<u8> {
    let ptr = ptr as *const u8;
    let mut offset = 0;
    while offset < layout.size {
        // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
        if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
            to = to.push(unsafe {
                ptr.wrapping_byte_offset((layout.size - offset - 1) as _)
                    .read()
            });
        } else {
            to = to.push(unsafe { ptr.wrapping_byte_offset(offset as _).read() });
        }
        offset += 1;
    }
    to
}

/// Serialize a constant sized array that is stored at the pointer passed in
const fn serialize_const_list(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &ListLayout,
) -> ConstVec<u8> {
    let len = layout.len;
    let mut i = 0;
    while i < len {
        let field = ptr.wrapping_byte_offset((i * layout.item_layout.size()) as _);
        to = serialize_const_ptr(field, to, layout.item_layout);
        i += 1;
    }
    to
}

/// Serialize a pointer to a type that is stored at the pointer passed in
const fn serialize_const_ptr(ptr: *const (), to: ConstVec<u8>, layout: &Layout) -> ConstVec<u8> {
    match layout {
        Layout::Enum(layout) => serialize_const_enum(ptr, to, layout),
        Layout::Struct(layout) => serialize_const_struct(ptr, to, layout),
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
/// let buf = buffer.read();
/// assert_eq!(buf.as_ref(), &[0x11, 0x11, 0x11, 0x11, 0x22, 0x33, 0x33, 0x33, 0x33]);
/// ```
#[must_use = "The data is serialized into the returned buffer"]
pub const fn serialize_const<T: SerializeConst>(data: &T, to: ConstVec<u8>) -> ConstVec<u8> {
    let ptr = data as *const T as *const ();
    serialize_const_ptr(ptr, to, &T::MEMORY_LAYOUT)
}

/// Deserialize a primitive type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_primitive<'a, const N: usize>(
    mut from: ConstReadBuffer<'a>,
    layout: &PrimitiveLayout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut offset = 0;
    while offset < layout.size {
        // If the bytes are reversed, walk backwards from the end of the number when filling in bytes
        let (from_new, value) = match from.get() {
            Some(data) => data,
            None => return None,
        };
        from = from_new;
        if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
            out[start + layout.size - offset - 1] = MaybeUninit::new(value);
        } else {
            out[start + offset] = MaybeUninit::new(value);
        }
        offset += 1;
    }
    Some((from, out))
}

/// Deserialize a struct type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_struct<'a, const N: usize>(
    mut from: ConstReadBuffer<'a>,
    layout: &StructLayout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut i = 0;
    while i < layout.data.len() {
        // Deserialize the field at the offset pointer in the struct
        let StructFieldLayout { offset, layout } = &layout.data[i];
        let (new_from, new_out) = match deserialize_const_ptr(from, layout, (start + *offset, out))
        {
            Some(data) => data,
            None => return None,
        };
        from = new_from;
        out = new_out;
        i += 1;
    }
    Some((from, out))
}

/// Deserialize an enum type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_enum<'a, const N: usize>(
    mut from: ConstReadBuffer<'a>,
    layout: &EnumLayout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut discriminant = 0;

    // First, deserialize the discriminant
    let mut offset = 0;
    while offset < layout.discriminant.size {
        // If the bytes are reversed, walk backwards from the end of the number when filling in bytes
        let (from_new, value) = match from.get() {
            Some(data) => data,
            None => return None,
        };
        from = from_new;
        if cfg!(target_endian = "big") {
            out[start + layout.size - offset - 1] = MaybeUninit::new(value);
            discriminant |= (value as u32) << ((layout.discriminant.size - offset - 1) * 8);
        } else {
            out[start + offset] = MaybeUninit::new(value);
            discriminant |= (value as u32) << (offset * 8);
        }
        offset += 1;
    }

    // Then, deserialize the variant
    let mut i = 0;
    let mut matched_variant = false;
    while i < layout.variants.len() {
        // If the variant is the discriminated one, deserialize it
        let EnumVariant { tag, data, .. } = &layout.variants[i];
        if discriminant == *tag {
            let offset = layout.variants_offset;
            let (new_from, new_out) =
                match deserialize_const_struct(from, data, (start + offset, out)) {
                    Some(data) => data,
                    None => return None,
                };
            from = new_from;
            out = new_out;
            matched_variant = true;
            break;
        }
        i += 1;
    }
    if !matched_variant {
        return None;
    }

    Some((from, out))
}

/// Deserialize a list type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_list<'a, const N: usize>(
    mut from: ConstReadBuffer<'a>,
    layout: &ListLayout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let len = layout.len;
    let item_layout = layout.item_layout;
    let mut i = 0;
    while i < len {
        let (new_from, new_out) =
            match deserialize_const_ptr(from, item_layout, (start + i * item_layout.size(), out)) {
                Some(data) => data,
                None => return None,
            };
        from = new_from;
        out = new_out;
        i += 1;
    }
    Some((from, out))
}

/// Deserialize a type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_ptr<'a, const N: usize>(
    from: ConstReadBuffer<'a>,
    layout: &Layout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    match layout {
        Layout::Enum(layout) => deserialize_const_enum(from, layout, out),
        Layout::Struct(layout) => deserialize_const_struct(from, layout, out),
        Layout::List(layout) => deserialize_const_list(from, layout, out),
        Layout::Primitive(layout) => deserialize_const_primitive(from, layout, out),
    }
}

/// Deserialize a type into the output buffer. Accepts (Type, ConstVec<u8>) as input and returns Option<(ConstReadBuffer, Instance of type)>
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
/// let buf = buffer.read();
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
    from: ConstReadBuffer,
) -> Option<(ConstReadBuffer, T)> {
    // Create uninitized memory with the size of the type
    let out = [MaybeUninit::uninit(); N];
    // Fill in the bytes into the buffer for the type
    let (from, out) = match deserialize_const_ptr(from, &T::MEMORY_LAYOUT, (0, out)) {
        Some(data) => data,
        None => return None,
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
