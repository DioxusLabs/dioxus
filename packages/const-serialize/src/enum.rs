use crate::*;

/// Serialize an enum that is stored at the pointer passed in
pub(crate) const unsafe fn serialize_const_enum(
    ptr: *const (),
    mut to: ConstVec<u8>,
    layout: &EnumLayout,
) -> ConstVec<u8> {
    let byte_ptr = ptr as *const u8;
    let discriminant = layout.discriminant.read(byte_ptr);

    let mut i = 0;
    while i < layout.variants.len() {
        // If the variant is the discriminated one, serialize it
        let EnumVariant {
            tag, name, data, ..
        } = &layout.variants[i];
        if discriminant == *tag {
            to = write_map(to, 1);
            to = write_map_key(to, name);
            let data_ptr = ptr.wrapping_byte_offset(layout.variants_offset as _);
            to = serialize_const_struct(data_ptr, to, data);
            break;
        }
        i += 1;
    }
    to
}

/// The layout for an enum. The enum layout is just a discriminate size and a tag layout.
#[derive(Debug, Copy, Clone)]
pub struct EnumLayout {
    pub(crate) size: usize,
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
    name: &'static str,
    // Note: tags may not be sequential
    tag: u32,
    data: StructLayout,
    align: usize,
}

impl EnumVariant {
    /// Create a new enum variant layout
    pub const fn new(name: &'static str, tag: u32, data: StructLayout, align: usize) -> Self {
        Self {
            name,
            tag,
            data,
            align,
        }
    }
}

/// Deserialize an enum type into the out buffer at the offset passed in. Returns a new version of the buffer with the data added.
pub(crate) const fn deserialize_const_enum<'a>(
    from: &'a [u8],
    layout: &EnumLayout,
    out: &mut [MaybeUninit<u8>],
) -> Option<&'a [u8]> {
    // First, deserialize the map
    let Ok((map, remaining)) = take_map(from) else {
        return None;
    };

    // Then get the only field which is the tag
    let Ok((deserilized_name, from)) = take_str(map.bytes) else {
        return None;
    };

    // Then, deserialize the variant
    let mut i = 0;
    let mut matched_variant = false;
    while i < layout.variants.len() {
        // If the variant is the discriminated one, deserialize it
        let EnumVariant {
            name, data, tag, ..
        } = &layout.variants[i];
        if str_eq(deserilized_name, name) {
            layout.discriminant.write(*tag, out);
            let Some((_, out)) = out.split_at_mut_checked(layout.variants_offset) else {
                return None;
            };
            if deserialize_const_struct(from, data, out).is_none() {
                return None;
            }
            matched_variant = true;
            break;
        }
        i += 1;
    }
    if !matched_variant {
        return None;
    }

    Some(remaining)
}
