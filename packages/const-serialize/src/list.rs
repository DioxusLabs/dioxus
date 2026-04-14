use crate::*;

/// The layout for a dynamically sized list. The list layout is just a length and an item layout.
#[derive(Debug, Copy, Clone)]
pub struct ListLayout {
    /// The size of the struct backing the list
    pub(crate) size: usize,
    /// The byte offset of the length field
    len_offset: usize,
    /// The layout of the length field
    len_layout: PrimitiveLayout,
    /// The byte offset of the data field
    data_offset: usize,
    /// The layout of the data field
    data_layout: ArrayLayout,
}

impl ListLayout {
    /// Create a new list layout
    pub const fn new(
        size: usize,
        len_offset: usize,
        len_layout: PrimitiveLayout,
        data_offset: usize,
        data_layout: ArrayLayout,
    ) -> Self {
        Self {
            size,
            len_offset,
            len_layout,
            data_offset,
            data_layout,
        }
    }
}

/// Serialize a dynamically sized list that is stored at the pointer passed in
pub(crate) const unsafe fn serialize_const_list(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &ListLayout,
) -> ConstVec<u8> {
    // Read the length of the list
    let len_ptr = ptr.wrapping_byte_offset(layout.len_offset as _);
    let len = layout.len_layout.read(len_ptr as *const u8) as usize;

    let data_ptr = ptr.wrapping_byte_offset(layout.data_offset as _);
    let item_layout = layout.data_layout.item_layout;
    // If the item size is 1, deserialize as bytes directly
    if item_layout.size() == 1 {
        let slice = std::slice::from_raw_parts(data_ptr as *const u8, len);
        to = write_bytes(to, slice);
    }
    // Otherwise, deserialize as a list of items
    else {
        let mut i = 0;
        to = write_array(to, len);
        while i < len {
            let item = data_ptr.wrapping_byte_offset((i * item_layout.size()) as _);
            to = serialize_const_ptr(item, to, item_layout);
            i += 1;
        }
    }
    to
}

/// Deserialize a list type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
pub(crate) const fn deserialize_const_list<'a>(
    from: &'a [u8],
    layout: &ListLayout,
    out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    let Some((_, len_out)) = out.split_at_mut_checked(layout.len_offset) else {
        return None;
    };

    // If the list items are only one byte, serialize as bytes directly
    let item_layout = layout.data_layout.item_layout;
    if item_layout.size() == 1 {
        let Ok((bytes, new_from)) = take_bytes(from) else {
            return None;
        };
        // Write out the length of the list
        layout.len_layout.write(bytes.len() as u32, len_out);
        let Some((_, data_out)) = out.split_at_mut_checked(layout.data_offset) else {
            return None;
        };
        let mut offset = 0;
        while offset < bytes.len() {
            data_out[offset] = MaybeUninit::new(bytes[offset]);
            offset += 1;
        }
        Some(new_from)
    }
    // Otherwise, serialize as an list of objects
    else {
        let Ok((len, mut from)) = take_array(from) else {
            return None;
        };
        // Write out the length of the list
        layout.len_layout.write(len as u32, len_out);
        let Some((_, mut data_out)) = out.split_at_mut_checked(layout.data_offset) else {
            return None;
        };
        let mut i = 0;
        while i < len {
            let Some(new_from) = deserialize_const_ptr(from, item_layout, data_out) else {
                return None;
            };
            let Some((_, item_out)) = data_out.split_at_mut_checked(item_layout.size()) else {
                return None;
            };
            data_out = item_out;
            from = new_from;
            i += 1;
        }
        Some(from)
    }
}
