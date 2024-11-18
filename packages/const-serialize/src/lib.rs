use std::mem::MaybeUninit;

mod const_buffers;
mod const_vec;

pub use const_buffers::{ConstReadBuffer, ConstWriteBuffer};
pub use const_serialize_macro::SerializeConst;

/// Plain old data for a field. Stores the offset of the field in the struct and the encoding of the field.
#[derive(Debug, Copy, Clone)]
pub struct StructFieldEncoding {
    offset: usize,
    encoding: Layout,
}

impl StructFieldEncoding {
    pub const fn new(offset: usize, encoding: Layout) -> Self {
        Self { offset, encoding }
    }
}

/// Layout for a struct. The struct encoding is just a list of fields with offsets
#[derive(Debug, Copy, Clone)]
pub struct StructEncoding {
    size: usize,
    data: &'static [StructFieldEncoding],
}

impl StructEncoding {
    pub const fn new(size: usize, data: &'static [StructFieldEncoding]) -> Self {
        Self { size, data }
    }
}

/// The encoding for an enum. The enum encoding is just a discriminate size and a tag encoding.
#[derive(Debug, Copy, Clone)]
pub struct EnumEncoding {
    size: usize,
    discriminant: PrimitiveEncoding,
    variants_offset: usize,
    variants: &'static [EnumVariant],
}

impl EnumEncoding {
    pub const fn new(
        size: usize,
        discriminant: PrimitiveEncoding,
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

        let variants_offset = (discriminant.size / max_align) + max_align;

        Self {
            size,
            discriminant,
            variants_offset,
            variants,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EnumVariant {
    // Note: tags may not be sequential
    tag: u32,
    data: StructEncoding,
    align: usize,
}

impl EnumVariant {
    pub const fn new(tag: u32, data: StructEncoding, align: usize) -> Self {
        Self { tag, data, align }
    }
}

/// The encoding for a constant sized array. The array encoding is just a length and an item encoding.
#[derive(Debug, Copy, Clone)]
pub struct ListEncoding {
    len: usize,
    item_encoding: &'static Layout,
}

impl ListEncoding {
    pub const fn new(len: usize, item_encoding: &'static Layout) -> Self {
        Self { len, item_encoding }
    }
}

/// The encoding for a primitive type. The bytes will be reversed if the target is big endian.
#[derive(Debug, Copy, Clone)]
pub struct PrimitiveEncoding {
    size: usize,
}

impl PrimitiveEncoding {
    pub const fn new(size: usize) -> Self {
        Self { size }
    }
}

/// The encoding for a type. This encoding defines a sequence of locations and reversed or not bytes. These bytes will be copied from during serialization and copied into during deserialization.
#[derive(Debug, Copy, Clone)]
pub enum Layout {
    Enum(EnumEncoding),
    Struct(StructEncoding),
    List(ListEncoding),
    Primitive(PrimitiveEncoding),
}

impl Layout {
    /// The size of the type in bytes.
    const fn size(&self) -> usize {
        match self {
            Layout::Enum(encoding) => encoding.size,
            Layout::Struct(encoding) => encoding.size,
            Layout::List(encoding) => encoding.len * encoding.item_encoding.size(),
            Layout::Primitive(encoding) => encoding.size,
        }
    }
}

/// A trait for types that can be serialized and deserialized in const.
///
/// # Safety
/// The encoding must accurately describe the memory layout of the type
pub unsafe trait SerializeConst: Sized {
    /// The memory layout of the type. This type must have plain old data; no pointers or references.
    const MEMORY_LAYOUT: Layout;
    const _ASSERT: () = assert!(Self::MEMORY_LAYOUT.size() == std::mem::size_of::<Self>());
}

macro_rules! impl_serialize_const {
    ($type:ty) => {
        unsafe impl SerializeConst for $type {
            const MEMORY_LAYOUT: Layout = Layout::Primitive(PrimitiveEncoding {
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
    const MEMORY_LAYOUT: Layout = Layout::List(ListEncoding {
        len: N,
        item_encoding: &T::MEMORY_LAYOUT,
    });
}

macro_rules! impl_serialize_const_tuple {
    ($($generic:ident: $generic_number:expr),*) => {
        impl_serialize_const_tuple!(@impl ($($generic,)*) = $($generic: $generic_number),*);
    };
    (@impl $inner:ty = $($generic:ident: $generic_number:expr),*) => {
        unsafe impl<$($generic: SerializeConst),*> SerializeConst for ($($generic,)*) {
            const MEMORY_LAYOUT: Layout = {
                Layout::Struct(StructEncoding {
                    size: std::mem::size_of::<($($generic,)*)>(),
                    data: &[
                        $(
                            StructFieldEncoding::new(std::mem::offset_of!($inner, $generic_number), $generic::MEMORY_LAYOUT),
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

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash)]
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
    const MEMORY_LAYOUT: Layout = Layout::Struct(StructEncoding {
        size: std::mem::size_of::<Self>(),
        data: &[
            StructFieldEncoding::new(
                std::mem::offset_of!(Self, bytes),
                Layout::List(ListEncoding {
                    len: MAX_STR_SIZE,
                    item_encoding: &Layout::Primitive(PrimitiveEncoding {
                        size: std::mem::size_of::<u8>(),
                    }),
                }),
            ),
            StructFieldEncoding::new(
                std::mem::offset_of!(Self, len),
                Layout::Primitive(PrimitiveEncoding {
                    size: std::mem::size_of::<u32>(),
                }),
            ),
        ],
    });
}

impl ConstStr {
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

    pub const fn as_str(&self) -> &str {
        let str_bytes = self.bytes.split_at(self.len as usize).0;
        match std::str::from_utf8(str_bytes) {
            Ok(s) => s,
            Err(_) => panic!(
                "Invalid utf8; ConstStr should only ever be constructed from valid utf8 strings"
            ),
        }
    }

    pub const fn push(self, byte: char) -> Self {
        assert!(byte.is_ascii(), "Only ASCII bytes are supported");
        let (bytes, len) = char_to_bytes(byte);
        let (str, _) = bytes.split_at(len);
        let Ok(str) = std::str::from_utf8(str) else {
            panic!("Invalid utf8; char_to_bytes should always return valid utf8 bytes")
        };
        self.push_str(str)
    }

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

#[test]
fn test_rsplit_once() {
    let str = ConstStr::new("hello");
    assert_eq!(
        str.rsplit_once('l'),
        Some((ConstStr::new("hel"), ConstStr::new("o")))
    );
    assert_eq!(
        str.rsplit_once('o'),
        Some((ConstStr::new("hell"), ConstStr::new("")))
    );
    assert_eq!(
        str.rsplit_once('e'),
        Some((ConstStr::new("h"), ConstStr::new("llo")))
    );

    let unicode_str = ConstStr::new("h😀ell😀o😀o");
    assert_eq!(
        unicode_str.rsplit_once('😀'),
        Some((ConstStr::new("h😀ell😀o"), ConstStr::new("o")))
    );
    assert_eq!(
        unicode_str.rsplit_once('o'),
        Some((ConstStr::new("h😀ell😀o😀"), ConstStr::new("")))
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
        0b00000000..0b10000000 => Some(1),
        0b11000000..0b11100000 => Some(2),
        0b11100000..0b11110000 => Some(3),
        0b11110000..0b11111000 => Some(4),
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
    mut to: ConstWriteBuffer,
    encoding: &StructEncoding,
) -> ConstWriteBuffer {
    let mut i = 0;
    while i < encoding.data.len() {
        // Serialize the field at the offset pointer in the struct
        let StructFieldEncoding { offset, encoding } = &encoding.data[i];
        let field = unsafe { ptr.byte_add(*offset) };
        to = serialize_const_ptr(field, to, encoding);
        i += 1;
    }
    to
}

/// Serialize an enum that is stored at the pointer passed in
const fn serialize_const_enum(
    ptr: *const (),
    mut to: ConstWriteBuffer,
    encoding: &EnumEncoding,
) -> ConstWriteBuffer {
    let mut discriminant = 0;

    let byte_ptr = ptr as *const u8;
    let mut offset = 0;
    while offset < encoding.discriminant.size {
        // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
        let byte = if cfg!(target_endian = "big") {
            unsafe {
                byte_ptr
                    .byte_add(encoding.discriminant.size - offset - 1)
                    .read()
            }
        } else {
            unsafe { byte_ptr.byte_add(offset).read() }
        };
        to = to.push(byte);
        discriminant |= (byte as u32) << (offset * 8);
        offset += 1;
    }

    let mut i = 0;
    while i < encoding.variants.len() {
        // If the variant is the discriminated one, serialize it
        let EnumVariant { tag, data, .. } = &encoding.variants[i];
        if discriminant == *tag {
            let data_ptr = unsafe { ptr.byte_add(encoding.variants_offset) };
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
    mut to: ConstWriteBuffer,
    encoding: &PrimitiveEncoding,
) -> ConstWriteBuffer {
    let ptr = ptr as *const u8;
    let mut offset = 0;
    while offset < encoding.size {
        // If the bytes are reversed, walk backwards from the end of the number when pushing bytes
        if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
            to = to.push(unsafe { ptr.byte_add(encoding.size - offset - 1).read() });
        } else {
            to = to.push(unsafe { ptr.byte_add(offset).read() });
        }
        offset += 1;
    }
    to
}

/// Serialize a constant sized array that is stored at the pointer passed in
const fn serialize_const_list(
    ptr: *const (),
    mut to: ConstWriteBuffer,
    encoding: &ListEncoding,
) -> ConstWriteBuffer {
    let len = encoding.len;
    let mut i = 0;
    while i < len {
        let field = unsafe { ptr.byte_add(i * encoding.item_encoding.size()) };
        to = serialize_const_ptr(field, to, encoding.item_encoding);
        i += 1;
    }
    to
}

/// Serialize a pointer to a type that is stored at the pointer passed in
const fn serialize_const_ptr(
    ptr: *const (),
    to: ConstWriteBuffer,
    encoding: &Layout,
) -> ConstWriteBuffer {
    match encoding {
        Layout::Enum(encoding) => serialize_const_enum(ptr, to, encoding),
        Layout::Struct(encoding) => serialize_const_struct(ptr, to, encoding),
        Layout::List(encoding) => serialize_const_list(ptr, to, encoding),
        Layout::Primitive(encoding) => serialize_const_primitive(ptr, to, encoding),
    }
}

/// Serialize a type into a buffer
#[must_use = "The data is serialized into the returned buffer"]
pub const fn serialize_const<T: SerializeConst>(
    data: &T,
    to: ConstWriteBuffer,
) -> ConstWriteBuffer {
    let ptr = data as *const T as *const ();
    serialize_const_ptr(ptr, to, &T::MEMORY_LAYOUT)
}

/// Deserialize a primitive type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
const fn deserialize_const_primitive<'a, const N: usize>(
    mut from: ConstReadBuffer<'a>,
    encoding: &PrimitiveEncoding,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut offset = 0;
    while offset < encoding.size {
        // If the bytes are reversed, walk backwards from the end of the number when filling in bytes
        let (from_new, value) = match from.get() {
            Some(data) => data,
            None => return None,
        };
        from = from_new;
        if cfg!(any(target_endian = "big", feature = "test-big-endian")) {
            out[start + encoding.size - offset - 1] = MaybeUninit::new(value);
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
    encoding: &StructEncoding,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut i = 0;
    while i < encoding.data.len() {
        // Deserialize the field at the offset pointer in the struct
        let StructFieldEncoding { offset, encoding } = &encoding.data[i];
        let (new_from, new_out) =
            match deserialize_const_ptr(from, encoding, (start + *offset, out)) {
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
    encoding: &EnumEncoding,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let mut discriminant = 0;

    // First, deserialize the discriminant
    let mut offset = 0;
    while offset < encoding.discriminant.size {
        // If the bytes are reversed, walk backwards from the end of the number when filling in bytes
        let (from_new, value) = match from.get() {
            Some(data) => data,
            None => return None,
        };
        from = from_new;
        if cfg!(target_endian = "big") {
            out[start + encoding.size - offset - 1] = MaybeUninit::new(value);
            discriminant |= (value as u32) << ((encoding.discriminant.size - offset - 1) * 8);
        } else {
            out[start + offset] = MaybeUninit::new(value);
            discriminant |= (value as u32) << (offset * 8);
        }
        offset += 1;
    }

    // Then, deserialize the variant
    let mut i = 0;
    let mut matched_variant = false;
    while i < encoding.variants.len() {
        // If the variant is the discriminated one, deserialize it
        let EnumVariant { tag, data, .. } = &encoding.variants[i];
        if discriminant == *tag {
            let offset = encoding.variants_offset;
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
    encoding: &ListEncoding,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    let (start, mut out) = out;
    let len = encoding.len;
    let item_encoding = encoding.item_encoding;
    let mut i = 0;
    while i < len {
        let (new_from, new_out) = match deserialize_const_ptr(
            from,
            item_encoding,
            (start + i * item_encoding.size(), out),
        ) {
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
    encoding: &Layout,
    out: (usize, [MaybeUninit<u8>; N]),
) -> Option<(ConstReadBuffer<'a>, [MaybeUninit<u8>; N])> {
    match encoding {
        Layout::Enum(encoding) => deserialize_const_enum(from, encoding, out),
        Layout::Struct(encoding) => deserialize_const_struct(from, encoding, out),
        Layout::List(encoding) => deserialize_const_list(from, encoding, out),
        Layout::Primitive(encoding) => deserialize_const_primitive(from, encoding, out),
    }
}

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
    let first_serialized = ConstWriteBuffer::new();
    let first_serialized = serialize_const(first, first_serialized);
    let second_serialized = ConstWriteBuffer::new();
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
