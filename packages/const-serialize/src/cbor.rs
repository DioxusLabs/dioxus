// Major type 0:
// An unsigned integer in the range 0..264-1 inclusive. The value of the encoded item is the argument itself. For example, the integer 10 is denoted as the one byte 0b000_01010 (major type 0, additional information 10). The integer 500 would be 0b000_11001 (major type 0, additional information 25) followed by the two bytes 0x01f4, which is 500 in decimal.
// Major type 1:
// A negative integer in the range -264..-1 inclusive. The value of the item is -1 minus the argument. For example, the integer -500 would be 0b001_11001 (major type 1, additional information 25) followed by the two bytes 0x01f3, which is 499 in decimal.
// Major type 2:
// A byte string. The number of bytes in the string is equal to the argument. For example, a byte string whose length is 5 would have an initial byte of 0b010_00101 (major type 2, additional information 5 for the length), followed by 5 bytes of binary content. A byte string whose length is 500 would have 3 initial bytes of 0b010_11001 (major type 2, additional information 25 to indicate a two-byte length) followed by the two bytes 0x01f4 for a length of 500, followed by 500 bytes of binary content.
// Major type 3:
// A text string (Section 2) encoded as UTF-8 [RFC3629]. The number of bytes in the string is equal to the argument. A string containing an invalid UTF-8 sequence is well-formed but invalid (Section 1.2). This type is provided for systems that need to interpret or display human-readable text, and allows the differentiation between unstructured bytes and text that has a specified repertoire (that of Unicode) and encoding (UTF-8). In contrast to formats such as JSON, the Unicode characters in this type are never escaped. Thus, a newline character (U+000A) is always represented in a string as the byte 0x0a, and never as the bytes 0x5c6e (the characters "\" and "n") nor as 0x5c7530303061 (the characters "\", "u", "0", "0", "0", and "a").
// Major type 4:
// An array of data items. In other formats, arrays are also called lists, sequences, or tuples (a "CBOR sequence" is something slightly different, though [RFC8742]). The argument is the number of data items in the array. Items in an array do not need to all be of the same type. For example, an array that contains 10 items of any type would have an initial byte of 0b100_01010 (major type 4, additional information 10 for the length) followed by the 10 remaining items.
// Major type 5:
// A map of pairs of data items. Maps are also called tables, dictionaries, hashes, or objects (in JSON). A map is comprised of pairs of data items, each pair consisting of a key that is immediately followed by a value. The argument is the number of pairs of data items in the map. For example, a map that contains 9 pairs would have an initial byte of 0b101_01001 (major type 5, additional information 9 for the number of pairs) followed by the 18 remaining items. The first item is the first key, the second item is the first value, the third item is the second key, and so on. Because items in a map come in pairs, their total number is always even: a map that contains an odd number of items (no value data present after the last key data item) is not well-formed. A map that has duplicate keys may be well-formed, but it is not valid, and thus it causes indeterminate decoding; see also Section 5.6.
// Major type 6:
// A tagged data item ("tag") whose tag number, an integer in the range 0..264-1 inclusive, is the argument and whose enclosed data item (tag content) is the single encoded data item that follows the head. See Section 3.4.
// Major type 7:
// Floating-point numbers and simple values, as well as the "break" stop code. See Section 3.3.

use crate::ConstVec;

#[repr(u8)]
#[derive(PartialEq)]
enum MajorType {
    UnsignedInteger = 0,
    NegativeInteger = 1,
    Bytes = 2,
    Text = 3,
    Array = 4,
    Map = 5,
    Tagged = 6,
    Float = 7,
}

impl MajorType {
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
    let additional_information = *head & 0b0001_1111;
    match major {
        MajorType::UnsignedInteger | MajorType::NegativeInteger => {
            Ok(1 + get_length_of_number(additional_information) as usize)
        }
        MajorType::Text | MajorType::Bytes => {
            let length_of_number = get_length_of_number(additional_information);
            let Ok((length_of_bytes, _)) =
                grab_u64_with_byte_length(rest, length_of_number, additional_information)
            else {
                return Err(());
            };
            Ok(1 + length_of_number as usize + length_of_bytes as usize)
        }
        MajorType::Array | MajorType::Map => {
            let length_of_number = get_length_of_number(additional_information);
            let Ok((length_of_items, _)) =
                grab_u64_with_byte_length(rest, length_of_number, additional_information)
            else {
                return Err(());
            };
            let mut total_length = length_of_number as usize + length_of_items as usize;
            let mut items_left = length_of_items;
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
            Ok(1 + total_length)
        }
        _ => Err(()),
    }
}

#[test]
fn test_item_length_str() {
    let input = [
        0x61, // text(1)
        /**/ 0x31, // "1"
        0x61, // text(1)
        /**/ 0x31, // "1"
    ];
    let Ok(length) = item_length(&input) else {
        panic!("Failed to calculate length");
    };
    assert_eq!(length, 2);
}

const fn take_number(bytes: &[u8]) -> Result<(i64, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & 0b0001_1111;
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

const fn write_number<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    number: i64,
) -> ConstVec<u8, MAX_SIZE> {
    match number {
        0.. => write_major_type_and_u64(vec, MajorType::UnsignedInteger, number as u64),
        ..0 => write_major_type_and_u64(vec, MajorType::NegativeInteger, (-(number + 1)) as u64),
    }
}

const fn write_major_type_and_u64<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    major: MajorType,
    number: u64,
) -> ConstVec<u8, MAX_SIZE> {
    let major = (major as u8) << 5;
    match number {
        0..24 => {
            let additional_information = number as u8;
            let byte = major | additional_information;
            vec.push(byte)
        }
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

const fn log2_bytes_for_number(number: u64) -> u8 {
    let required_bytes = ((64 - number.leading_zeros()).div_ceil(8)) as u8;
    match required_bytes {
        ..=1 => 0,
        ..=2 => 1,
        ..=4 => 2,
        _ => 3,
    }
}

const fn take_bytes(bytes: &[u8]) -> Result<(&[u8], &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & 0b0001_1111;
    if let MajorType::Bytes = major {
        take_bytes_from(rest, additional_information)
    } else {
        Err(())
    }
}

const fn write_bytes<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    bytes: &[u8],
) -> ConstVec<u8, MAX_SIZE> {
    let vec = write_major_type_and_u64(vec, MajorType::Bytes, bytes.len() as u64);
    vec.extend(bytes)
}

const fn take_str(bytes: &[u8]) -> Result<(&str, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & 0b0001_1111;
    if let MajorType::Text = major {
        let Ok((bytes, rest)) = take_bytes_from(rest, additional_information) else {
            return Err(());
        };
        let Ok(string) = str::from_utf8(bytes) else {
            return Err(());
        };
        Ok((string, rest))
    } else {
        Err(())
    }
}

const fn write_str<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    string: &str,
) -> ConstVec<u8, MAX_SIZE> {
    let vec = write_major_type_and_u64(vec, MajorType::Text, string.len() as u64);
    vec.extend(string.as_bytes())
}

const fn take_array(bytes: &[u8]) -> Result<(usize, &[u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & 0b0001_1111;
    if let MajorType::Array = major {
        let Ok((length, rest)) = take_len_from(rest, additional_information) else {
            return Err(());
        };
        Ok((length as usize, rest))
    } else {
        Err(())
    }
}

const fn write_array<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    len: usize,
) -> ConstVec<u8, MAX_SIZE> {
    write_major_type_and_u64(vec, MajorType::Array, len as u64)
}

const fn write_map<const MAX_SIZE: usize>(
    vec: ConstVec<u8, MAX_SIZE>,
    len: usize,
) -> ConstVec<u8, MAX_SIZE> {
    // We write 2 * len as the length of the map because each key-value pair is a separate entry.
    write_major_type_and_u64(vec, MajorType::Map, len as u64)
}

const fn write_map_key<const MAX_SIZE: usize>(
    value: ConstVec<u8, MAX_SIZE>,
    key: &str,
) -> ConstVec<u8, MAX_SIZE> {
    write_str(value, key)
}

const fn take_map<'a>(bytes: &'a [u8]) -> Result<(MapRef<'a>, &'a [u8]), ()> {
    let [head, rest @ ..] = bytes else {
        return Err(());
    };
    let major = MajorType::from_byte(*head);
    let additional_information = *head & 0b0001_1111;
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
            let Some((_, rest)) = after_map.split_at_checked(len as usize) else {
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

struct MapRef<'a> {
    bytes: &'a [u8],
    len: usize,
}

impl<'a> MapRef<'a> {
    const fn new(bytes: &'a [u8], len: usize) -> Self {
        Self { bytes, len }
    }

    const fn find(&self, key: &str) -> Result<Option<&[u8]>, ()> {
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
            let Some((_, rest)) = rest.split_at_checked(len as usize) else {
                return Err(());
            };
            bytes = rest;
            items_left -= 1;
        }
        Ok(None)
    }
}

const fn str_eq(a: &str, b: &str) -> bool {
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

const fn take_len_from(rest: &[u8], additional_information: u8) -> Result<(u64, &[u8]), ()> {
    match additional_information {
        // If additional_information < 24, the argument's value is the value of the additional information.
        0..24 => Ok((additional_information as u64, rest)),
        // If additional_information is between 24 and 28, the argument's value is held in the n following bytes.
        24..28 => {
            let Ok((number, rest)) = grab_u64(rest, additional_information) else {
                return Err(());
            };
            Ok((number as u64, rest))
        }
        _ => Err(()),
    }
}

const fn take_bytes_from(rest: &[u8], additional_information: u8) -> Result<(&[u8], &[u8]), ()> {
    let Ok((number, rest)) = grab_u64(rest, additional_information) else {
        return Err(());
    };
    let Some((bytes, rest)) = rest.split_at_checked(number as usize) else {
        return Err(());
    };
    Ok((bytes, rest))
}

const fn get_length_of_number(additional_information: u8) -> u8 {
    match additional_information {
        0..24 => 0,
        24..28 => 1 << (additional_information - 24),
        _ => 0,
    }
}

const fn grab_u64(rest: &[u8], additional_information: u8) -> Result<(u64, &[u8]), ()> {
    grab_u64_with_byte_length(
        rest,
        get_length_of_number(additional_information),
        additional_information,
    )
}

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

#[test]
fn test_parse_byte() {
    for byte in 0..=255 {
        let bytes = if byte < 24 {
            [byte | 0b00000000, 0]
        } else {
            [0b00000000 | 24, byte]
        };
        let (item, _) = take_number(&bytes).unwrap();
        assert_eq!(item, byte as _);
    }
    for byte in 1..=255 {
        let bytes = if byte < 24 {
            [byte - 1 | 0b0010_0000, 0]
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
