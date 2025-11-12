use crate::*;

/// The layout for a constant sized array. The array layout is just a length and an item layout.
#[derive(Debug, Copy, Clone)]
pub struct ArrayLayout {
    pub(crate) len: usize,
    pub(crate) item_layout: &'static Layout,
}

impl ArrayLayout {
    /// Create a new array layout
    pub const fn new(len: usize, item_layout: &'static Layout) -> Self {
        Self { len, item_layout }
    }
}

unsafe impl<const N: usize, T: SerializeConst> SerializeConst for [T; N] {
    const MEMORY_LAYOUT: Layout = Layout::Array(ArrayLayout {
        len: N,
        item_layout: &T::MEMORY_LAYOUT,
    });
}

/// Serialize a constant sized array that is stored at the pointer passed in
pub(crate) const unsafe fn serialize_const_array(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &ArrayLayout,
) -> ConstVec<u8> {
    let len = layout.len;
    let mut i = 0;
    to = write_array(to, len);
    while i < len {
        let field = ptr.wrapping_byte_offset((i * layout.item_layout.size()) as _);
        to = serialize_const_ptr(field, to, layout.item_layout);
        i += 1;
    }
    to
}

/// Deserialize an array type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
pub(crate) const fn deserialize_const_array<'a>(
    from: &'a [u8],
    layout: &ArrayLayout,
    mut out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    let item_layout = layout.item_layout;
    let Ok((_, mut from)) = take_array(from) else {
        return None;
    };
    let mut i = 0;
    while i < layout.len {
        let Some(new_from) = deserialize_const_ptr(from, item_layout, out) else {
            return None;
        };
        let Some((_, item_out)) = out.split_at_mut_checked(item_layout.size()) else {
            return None;
        };
        out = item_out;
        from = new_from;
        i += 1;
    }
    Some(from)
}
