use std::mem::MaybeUninit;

mod const_buffers;
mod const_vec;

pub use const_buffers::{ConstReadBuffer, ConstWriteBuffer};
pub use derive_const_serialize::SerializeConst;

/// Plain old data for a field. Stores the offset of the field in the struct and the encoding of the field.
#[derive(Debug, Copy, Clone)]
pub struct PlainOldData {
    offset: usize,
    encoding: Layout,
}

impl PlainOldData {
    pub const fn new(offset: usize, encoding: Layout) -> Self {
        Self { offset, encoding }
    }
}

/// Layout for a struct. The struct encoding is just a list of fields with offsets
#[derive(Debug, Copy, Clone)]
pub struct StructEncoding {
    size: usize,
    data: &'static [PlainOldData],
}

impl StructEncoding {
    pub const fn new(size: usize, data: &'static [PlainOldData]) -> Self {
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
                            PlainOldData::new(std::mem::offset_of!($inner, $generic_number), $generic::MEMORY_LAYOUT),
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

/// Serialize a struct that is stored at the pointer passed in
const fn serialize_const_struct(
    ptr: *const (),
    mut to: ConstWriteBuffer,
    encoding: &StructEncoding,
) -> ConstWriteBuffer {
    let mut i = 0;
    while i < encoding.data.len() {
        // Serialize the field at the offset pointer in the struct
        let PlainOldData { offset, encoding } = &encoding.data[i];
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
        let PlainOldData { offset, encoding } = &encoding.data[i];
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
) -> Option<T> {
    // Create uninitized memory with the size of the type
    let out = [MaybeUninit::uninit(); N];
    // Fill in the bytes into the buffer for the type
    let (_, out) = match deserialize_const_ptr(from, &T::MEMORY_LAYOUT, (0, out)) {
        Some(data) => data,
        None => return None,
    };
    // Now that the memory is filled in, transmute it into the type
    Some(unsafe { std::mem::transmute_copy::<[MaybeUninit<u8>; N], T>(&out) })
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
