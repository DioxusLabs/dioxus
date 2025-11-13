use crate::*;

/// Plain old data for a field. Stores the offset of the field in the struct and the layout of the field.
#[derive(Debug, Copy, Clone)]
pub struct StructFieldLayout {
    name: &'static str,
    offset: usize,
    layout: Layout,
}

impl StructFieldLayout {
    /// Create a new struct field layout
    pub const fn new(name: &'static str, offset: usize, layout: Layout) -> Self {
        Self {
            name,
            offset,
            layout,
        }
    }
}

/// Layout for a struct. The struct layout is just a list of fields with offsets
#[derive(Debug, Copy, Clone)]
pub struct StructLayout {
    pub(crate) size: usize,
    pub(crate) data: &'static [StructFieldLayout],
}

impl StructLayout {
    /// Create a new struct layout
    pub const fn new(size: usize, data: &'static [StructFieldLayout]) -> Self {
        Self { size, data }
    }
}

/// Serialize a struct that is stored at the pointer passed in
pub(crate) const unsafe fn serialize_const_struct(
    ptr: *const (),
    to: ConstVec<u8>,
    layout: &StructLayout,
) -> ConstVec<u8> {
    let mut i = 0;
    let field_count = layout.data.len();
    let mut to = write_map(to, field_count);
    while i < field_count {
        // Serialize the field at the offset pointer in the struct
        let StructFieldLayout {
            name,
            offset,
            layout,
        } = &layout.data[i];
        to = write_map_key(to, name);
        let field = ptr.wrapping_byte_add(*offset as _);
        to = serialize_const_ptr(field, to, layout);
        i += 1;
    }
    to
}

/// Deserialize a struct type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
pub(crate) const fn deserialize_const_struct<'a>(
    from: &'a [u8],
    layout: &StructLayout,
    out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    let Ok((map, from)) = take_map(from) else {
        return None;
    };
    let mut i = 0;
    while i < layout.data.len() {
        // Deserialize the field at the offset pointer in the struct
        let StructFieldLayout {
            name,
            offset,
            layout,
        } = &layout.data[i];
        let Ok(Some(from)) = map.find(name) else {
            return None;
        };
        let Some((_, field_bytes)) = out.split_at_mut_checked(*offset) else {
            return None;
        };
        if deserialize_const_ptr(from, layout, field_bytes).is_none() {
            return None;
        }
        i += 1;
    }
    Some(from)
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
                            StructFieldLayout::new(stringify!($generic_number), std::mem::offset_of!($inner, $generic_number), $generic::MEMORY_LAYOUT),
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
