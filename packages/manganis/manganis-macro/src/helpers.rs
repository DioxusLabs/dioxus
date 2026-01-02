/// Copy a slice into a constant sized buffer at compile time
///
/// This is a generic utility that works with any byte slice and can be used
/// in const contexts to create fixed-size arrays from dynamic slices.
pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

/// Serialize a value to a const buffer, padding to the specified size
///
/// This is a generic helper that works with any type implementing `SerializeConst`.
/// It serializes the value and then pads the buffer to the specified memory layout size.
pub const fn serialize_to_const<T: SerializeConst>(
    value: &T,
    memory_layout_size: usize,
) -> ConstVec<u8> {
    let data = ConstVec::new();
    let mut data = const_serialize::serialize_const(value, data);
    // Reserve the maximum size of the type
    while data.len() < memory_layout_size {
        data = data.push(0);
    }
    data
}

/// Serialize a value to a const buffer with a fixed maximum size, padding to the specified size
///
/// This variant uses a `ConstVec` with a fixed maximum size (e.g., `ConstVec<u8, 4096>`)
/// and then pads to the specified memory layout size.
///
/// This function serializes directly into the larger buffer to avoid overflow issues
/// when the serialized data exceeds the default 1024-byte buffer size.
pub const fn serialize_to_const_with_max<const MAX_SIZE: usize>(
    value: &impl SerializeConst,
    memory_layout_size: usize,
) -> ConstVec<u8, MAX_SIZE> {
    // Serialize using the default buffer, then copy into the larger buffer
    let serialized = const_serialize::serialize_const(value, ConstVec::new());
    let mut data: ConstVec<u8, MAX_SIZE> = ConstVec::new_with_max_size();
    data = data.extend(serialized.as_ref());
    // Reserve the maximum size of the type (pad to MEMORY_LAYOUT size)
    while data.len() < memory_layout_size {
        data = data.push(0);
    }
    data
}

/// Serialize a value to a const buffer and pad to the full buffer size
///
/// This is useful for linker section generation that expects a fixed-size buffer.
pub const fn serialize_to_const_with_max_padded<const MAX_SIZE: usize>(
    value: &impl SerializeConst,
) -> ConstVec<u8, MAX_SIZE> {
    let serialized = const_serialize::serialize_const(value, ConstVec::new());
    let mut data: ConstVec<u8, MAX_SIZE> = ConstVec::new_with_max_size();
    data = data.extend(serialized.as_ref());
    while data.len() < MAX_SIZE {
        data = data.push(0);
    }
    data
}

/// Serialize a value using the legacy 0.7 const-serialize format and pad to layout size
pub const fn serialize_to_const_with_layout_padded_07<T: const_serialize_07::SerializeConst>(
    value: &T,
) -> const_serialize_07::ConstVec<u8> {
    let data = const_serialize_07::ConstVec::new();
    let mut data = const_serialize_07::serialize_const(value, data);
    while data.len() < T::MEMORY_LAYOUT.size() {
        data = data.push(0);
    }
    data
}
