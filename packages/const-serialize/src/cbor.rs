//! Const serialization utilities for the CBOR data format.
//!
//! ## Overview of the format
//!
//! Const serialize only supports a subset of the CBOR format, specifically the major types:
//! - UnsignedInteger
//! - NegativeInteger
//! - Bytes
//! - String
//! - Array
//!
//! Each item in CBOR starts with a leading byte, which determines the type of the item and additional information.
//! The additional information is encoded in the lower 5 bits of the leading byte and generally indicates either a
//! small number or how many of the next bytes are part of the first number.
//!
//! Resources:
//! The spec: <https://www.rfc-editor.org/rfc/rfc8949.html>
//! A playground to check examples against: <https://cbor.me/>

use crate::ConstVec;

/// Each item in CBOR starts with a leading byte, which determines the type of the item and additional information.
///
/// The first 3 bits of the leading byte are the major type, which indicates the type of the item.
#[repr(u8)]
#[derive(PartialEq)]
enum MajorType {
    /// An unsigned integer in the range 0..2^64. The value of the number is encoded in the remaining bits of the leading byte and any additional bytes.
    UnsignedInteger = 0,
    /// An unsigned integer in the range -2^64..-1. The value of the number is encoded in the remaining bits of the leading byte and any additional bytes
    NegativeInteger = 1,
    /// A byte sequence. The number of bytes in the sequence is encoded in the remaining bits of the leading byte and any additional bytes.
    Bytes = 2,
    /// A text sequence. The number of bytes in the sequence is encoded in the remaining bits of the leading byte and any additional bytes.
    Text = 3,
    /// A dynamically sized array of non-uniform data items. The number of items in the array is encoded in the remaining bits of the leading byte and any additional bytes.
    Array = 4,
    /// A map of pairs of data items. The first item in each pair is the key and the second item is the value. The number of items in the array is encoded in the remaining bits of the leading byte and any additional bytes.
    Map = 5,
    /// Tagged values - not supported
    Tagged = 6,
    /// Floating point values - not supported
    Float = 7,
}

impl MajorType {
    /// The bitmask for the major type in the leading byte
    const MASK: u8 = 0b0001_1111;

    const fn from_byte(byte: u8) -> Self {
        match byte >> 5 {
            0 => MajorType::UnsignedInteger,
            1 => MajorType::NegativeInteger,
            2 => MajorType::Bytes,
            3 => MajorType::Text,
            4 => MajorType::Array,
            5 => MajorType::Map,
            6 => MajorType::Tagged,
            7 => MajorType::Float,
            _ => panic!("Invalid major type"),
        }
    }
}

/// Get the length of the item in bytes without deserialization.
const fn item_length(bytes: &[u8]) -> Result<usize, ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    let length_of_item = match major {
        // The length of the number is the total of:
        // - The length of the number (which may be 0 if the number is encoded in additional information)
        MajorType::UnsignedInteger | MajorType::NegativeInteger => {
            get_length_of_number(additional_information) as usize
        }
        // The length of the text or bytes is the total of:
        // - The length of the number that denotes the length of the text or bytes
        // - The length of the text or bytes themselves
        MajorType::Text | MajorType::Bytes => {
            let length_of_number = get_length_of_number(additional_information);
            let Ok((length_of_bytes, _)) =
                grab_u64_with_byte_length(rest, length_of_number, additional_information)
            else {
                return Err(());
            };
            length_of_number as usize + length_of_bytes as usize
        }
        // The length of the map is the total of:
        // - The length of the number that denotes the number of items
        // - The length of the pairs of items themselves
        MajorType::Array | MajorType::Map => {
            let length_of_number = get_length_of_number(additional_information);
            let Ok((length_of_items, _)) =
                grab_u64_with_byte_length(rest, length_of_number, additional_information)
            else {
                return Err(());
            };
            let mut total_length = length_of_number as usize;
            let mut items_left = length_of_items * if let MajorType::Map = major { 2 } else { 1 };
            while items_left > 0 {
                let Some((_, after)) = rest.split_at_checked(total_length) else {
                    return Err(());
                };
                let Ok(item_length) = item_length(after) else {
                    return Err(());
                };
                total_length += item_length;
                items_left -= 1;
            }
            total_length
        }
        _ => return Err(()),
    };
    let length_of_head = 1;
    Ok(length_of_head + length_of_item)
}

/// Read a number from the buffer, returning the number and the remaining bytes.
pub(crate) const fn take_number(bytes: &[u8]) -> Result<(i64, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    match major {
        MajorType::UnsignedInteger => {
            let Ok((number, rest)) = grab_u64(rest, additional_information) else {
                return Err(());
            };
            Ok((number as i64, rest))
        }
        MajorType::NegativeInteger => {
            let Ok((number, rest)) = grab_u64(rest, additional_information) else {
                return Err(());
            };
            Ok((-(1 + number as i64), rest))
        }
        _ => Err(()),
    }
}

/// Write a number to the buffer
pub(crate) const fn write_number<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    number: i64,
) -> ConstVec<u8, MAX_SIZE> {
    match number {
        0.. => write_major_type_and_u64(vec, MajorType::UnsignedInteger, number as u64),
        ..0 => write_major_type_and_u64(vec, MajorType::NegativeInteger, (-(number + 1)) as u64),
    }
}

/// Write the major type along with a number to the buffer. The first byte
/// contains both the major type and the additional information which contains
/// either the number itself or the number of extra bytes the number occupies.
const fn write_major_type_and_u64<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    major: MajorType,
    number: u64,
) -> ConstVec<u8, MAX_SIZE> {
    let major = (major as u8) << 5;
    match number {
        // For numbers less than 24, store the number in the lower bits
        // of the first byte
        0..24 => {
            let additional_information = number as u8;
            let byte = major | additional_information;
            vec.push(byte)
        }
        // For larger numbers, store the number of extra bytes the number occupies
        24.. => {
            let log2_additional_bytes = log2_bytes_for_number(number);
            let additional_bytes = 1 << log2_additional_bytes;
            let additional_information = log2_additional_bytes + 24;
            let byte = major | additional_information;
            let mut vec = vec.push(byte);
            let mut byte = 0;
            while byte < additional_bytes {
                vec = vec.push((number >> ((additional_bytes - byte - 1) * 8)) as u8);
                byte += 1;
            }
            vec
        }
    }
}

/// Find the number of bytes required to store a number and return the log2 of the number of bytes.
/// This is the number stored in the additional information field if the number is more than 24.
const fn log2_bytes_for_number(number: u64) -> u8 {
    let required_bytes = ((64 - number.leading_zeros()).div_ceil(8)) as u8;
    #[allow(clippy::match_overlapping_arm)]
    match required_bytes {
        ..=1 => 0,
        ..=2 => 1,
        ..=4 => 2,
        _ => 3,
    }
}

/// Take bytes from a slice and return the bytes and the remaining slice.
pub(crate) const fn take_bytes(bytes: &[u8]) -> Result<(&[u8], &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    if let MajorType::Bytes = major {
        take_bytes_from(rest, additional_information)
    } else {
        Err(())
    }
}

/// Write bytes to a buffer and return the new buffer.
pub(crate) const fn write_bytes<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    bytes: &[u8],
) -> ConstVec<u8, MAX_SIZE> {
    let vec = write_major_type_and_u64(vec, MajorType::Bytes, bytes.len() as u64);
    vec.extend(bytes)
}

/// Take a string from a buffer and return the string and the remaining buffer.
pub(crate) const fn take_str(bytes: &[u8]) -> Result<(&str, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    if let MajorType::Text = major {
        let Ok((bytes, rest)) = take_bytes_from(rest, additional_information) else {
            return Err(());
        };
        let Ok(string) = std::str::from_utf8(bytes) else {
            return Err(());
        };
        Ok((string, rest))
    } else {
        Err(())
    }
}

/// Write a string to a buffer and return the new buffer.
pub(crate) const fn write_str<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    string: &str,
) -> ConstVec<u8, MAX_SIZE> {
    let vec = write_major_type_and_u64(vec, MajorType::Text, string.len() as u64);
    vec.extend(string.as_bytes())
}

/// Take the length and header of an array from a buffer and return the length and the remaining buffer.
/// You must loop over the elements of the array and parse them outside of this method.
pub(crate) const fn take_array(bytes: &[u8]) -> Result<(usize, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    if let MajorType::Array = major {
        let Ok((length, rest)) = take_len_from(rest, additional_information) else {
            return Err(());
        };
        Ok((length as usize, rest))
    } else {
        Err(())
    }
}

/// Write the header and length of an array.
pub(crate) const fn write_array<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    len: usize,
) -> ConstVec<u8, MAX_SIZE> {
    write_major_type_and_u64(vec, MajorType::Array, len as u64)
}

/// Write the header and length of a map.
pub(crate) const fn write_map<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    len: usize,
) -> ConstVec<u8, MAX_SIZE> {
    // We write 2 * len as the length of the map because each key-value pair is a separate entry.
    write_major_type_and_u64(vec, MajorType::Map, len as u64)
}

/// Write the key of a map entry.
pub(crate) const fn write_map_key<const MAX_SIZE: usize>(
    value: ConstVec<u8, MAX_SIZE>,
    key: &str,
) -> ConstVec<u8, MAX_SIZE> {
    write_str(value, key)
}

/// Take a map from the byte slice and return the map reference and the remaining bytes.
pub(crate) const fn take_map<'a>(bytes: &'a [u8]) -> Result<(MapRef<'a>, &'a [u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & MajorType::MASK;
    if let MajorType::Map = major {
        let Ok((length, rest)) = take_len_from(rest, additional_information) else {
            return Err(());
        };
        let mut after_map = rest;
        let mut items_left = length * 2;
        while items_left > 0 {
            // Skip the value
            let Ok(len) = item_length(after_map) else {
                return Err(());
            };
            let Some((_, rest)) = after_map.split_at_checked(len) else {
                return Err(());
            };
            after_map = rest;
            items_left -= 1;
        }
        Ok((MapRef::new(rest, length as usize), after_map))
    } else {
        Err(())
    }
}

/// A reference to a CBOR map.
pub(crate) struct MapRef<'a> {
    /// The bytes of the map.
    pub(crate) bytes: &'a [u8],
    /// The length of the map.
    pub(crate) len: usize,
}

impl<'a> MapRef<'a> {
    /// Create a new map reference.
    const fn new(bytes: &'a [u8], len: usize) -> Self {
        Self { bytes, len }
    }

    /// Find a key in the map and return the buffer associated with it.
    pub(crate) const fn find(&self, key: &str) -> Result<Option<&[u8]>, ()> {
        let mut bytes = self.bytes;
        let mut items_left = self.len;
        while items_left > 0 {
            let Ok((str, rest)) = take_str(bytes) else {
                return Err(());
            };
            if str_eq(key, str) {
                return Ok(Some(rest));
            }
            // Skip the value associated with the key we don't care about
            let Ok(len) = item_length(rest) else {
                return Err(());
            };
            let Some((_, rest)) = rest.split_at_checked(len) else {
                return Err(());
            };
            bytes = rest;
            items_left -= 1;
        }
        Ok(None)
    }
}

/// Compare two strings for equality at compile time.
pub(crate) const fn str_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let a_len = a_bytes.len();
    let b_len = b_bytes.len();
    if a_len != b_len {
        return false;
    }
    let mut index = 0;
    while index < a_len {
        if a_bytes[index] != b_bytes[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Take the length from the additional information byte and return it along with the remaining bytes.
const fn take_len_from(rest: &[u8], additional_information: u8) -> Result<(u64, &[u8]), ()> {
    match additional_information {
        // If additional_information < 24, the argument's value is the value of the additional information.
        0..24 => Ok((additional_information as u64, rest)),
        // If additional_information is between 24 and 28, the argument's value is held in the n following bytes.
        24..28 => {
            let Ok((number, rest)) = grab_u64(rest, additional_information) else {
                return Err(());
            };
            Ok((number, rest))
        }
        _ => Err(()),
    }
}

/// Take a list of bytes from the byte slice and the additional information byte
/// and return the bytes and the remaining bytes.
pub(crate) const fn take_bytes_from(
    rest: &[u8],
    additional_information: u8,
) -> Result<(&[u8], &[u8]), ()> {
    let Ok((number, rest)) = grab_u64(rest, additional_information) else {
        return Err(());
    };
    let Some((bytes, rest)) = rest.split_at_checked(number as usize) else {
        return Err(());
    };
    Ok((bytes, rest))
}

/// Find the length of the number based on the additional information byte.
const fn get_length_of_number(additional_information: u8) -> u8 {
    match additional_information {
        0..24 => 0,
        24..28 => 1 << (additional_information - 24),
        _ => 0,
    }
}

/// Read a u64 from the byte slice and the additional information byte.
const fn grab_u64(rest: &[u8], additional_information: u8) -> Result<(u64, &[u8]), ()> {
    grab_u64_with_byte_length(
        rest,
        get_length_of_number(additional_information),
        additional_information,
    )
}

/// Read a u64 from the byte slice and the additional information byte along with the byte length.
const fn grab_u64_with_byte_length(
    mut rest: &[u8],
    byte_length: u8,
    additional_information: u8,
) -> Result<(u64, &[u8]), ()> {
    match byte_length {
        0 => Ok((additional_information as u64, rest)),
        n => {
            let mut value = 0;
            let mut count = 0;
            while count < n {
                let [next, remaining @ ..] = rest else {
                    return Err(());
                };
                value = (value << 8) | *next as u64;
                rest = remaining;
                count += 1;
            }
            Ok((value, rest))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_byte() {
        for byte in 0..=255 {
            let bytes = if byte < 24 { [byte, 0] } else { [24, byte] };
            let (item, _) = take_number(&bytes).unwrap();
            assert_eq!(item, byte as _);
        }
        for byte in 1..=255 {
            let bytes = if byte < 24 {
                [(byte - 1) | 0b0010_0000, 0]
            } else {
                [0b0010_0000 | 24, byte - 1]
            };
            let (item, _) = take_number(&bytes).unwrap();
            assert_eq!(item, -(byte as i64));
        }
    }

    #[test]
    fn test_byte_roundtrip() {
        for byte in 0..=255 {
            let vec = write_number(ConstVec::new(), byte as _);
            println!("{vec:?}");
            let (item, _) = take_number(vec.as_ref()).unwrap();
            assert_eq!(item, byte as _);
        }
        for byte in 0..=255 {
            let vec = write_number(ConstVec::new(), -(byte as i64));
            let (item, _) = take_number(vec.as_ref()).unwrap();
            assert_eq!(item, -(byte as i64));
        }
    }

    #[test]
    fn test_number_roundtrip() {
        for _ in 0..100 {
            let value = rand::random::<i64>();
            let vec = write_number(ConstVec::new(), value);
            let (item, _) = take_number(vec.as_ref()).unwrap();
            assert_eq!(item, value);
        }
    }

    #[test]
    fn test_bytes_roundtrip() {
        for _ in 0..100 {
            let len = (rand::random::<u8>() % 100) as usize;
            let bytes = rand::random::<[u8; 100]>();
            let vec = write_bytes(ConstVec::new(), &bytes[..len]);
            let (item, _) = take_bytes(vec.as_ref()).unwrap();
            assert_eq!(item, &bytes[..len]);
        }
    }

    #[test]
    fn test_array_roundtrip() {
        for _ in 0..100 {
            let len = (rand::random::<u8>() % 100) as usize;
            let mut vec = write_array(ConstVec::new(), len);
            for i in 0..len {
                vec = write_number(vec, i as _);
            }
            let (len, mut remaining) = take_array(vec.as_ref()).unwrap();
            for i in 0..len {
                let (item, rest) = take_number(remaining).unwrap();
                remaining = rest;
                assert_eq!(item, i as i64);
            }
        }
    }

    #[test]
    fn test_map_roundtrip() {
        use rand::prelude::SliceRandom;
        for _ in 0..100 {
            let len = (rand::random::<u8>() % 10) as usize;
            let mut vec = write_map(ConstVec::new(), len);
            let mut random_order_indexes = (0..len).collect::<Vec<_>>();
            random_order_indexes.shuffle(&mut rand::rng());
            for &i in &random_order_indexes {
                vec = write_map_key(vec, &i.to_string());
                vec = write_number(vec, i as _);
            }
            println!("len: {}", len);
            println!("Map: {:?}", vec);
            let (map, remaining) = take_map(vec.as_ref()).unwrap();
            println!("remaining: {:?}", remaining);
            assert!(remaining.is_empty());
            for i in 0..len {
                let key = i.to_string();
                let key_location = map
                    .find(&key)
                    .expect("encoding is valid")
                    .expect("key exists");
                let (value, _) = take_number(key_location).unwrap();
                assert_eq!(value, i as i64);
            }
        }
    }

    #[test]
    fn test_item_length_str() {
        #[rustfmt::skip]
    let input = [
        /* text(1) */ 0x61,
        /* "1" */     0x31,
        /* text(1) */ 0x61,
        /* "1" */     0x31,
    ];
        let Ok(length) = item_length(&input) else {
            panic!("Failed to calculate length");
        };
        assert_eq!(length, 2);
    }

    #[test]
    fn test_item_length_map() {
        #[rustfmt::skip]
    let input = [
        /* map(1) */              0xA1,
        /* text(1) */             0x61,
        /* "A" */                 0x41,
        /* map(2) */              0xA2,
        /* text(3) */             0x63,
        /* "one" */               0x6F, 0x6E, 0x65,
        /* unsigned(286331153) */ 0x1A, 0x11, 0x11, 0x11, 0x11,
        /* text(3) */             0x63,
        /* "two" */               0x74, 0x77, 0x6F,
        /* unsigned(34) */        0x18, 0x22,
    ];
        let Ok(length) = item_length(&input) else {
            panic!("Failed to calculate length");
        };
        assert_eq!(length, input.len());
    }
}
