use crate::*;
use std::{char, fmt::Debug, hash::Hash, mem::MaybeUninit};

const MAX_STR_SIZE: usize = 256;

/// A string that is stored in a constant sized buffer that can be serialized and deserialized at compile time
#[derive(Clone, Copy)]
pub struct ConstStr {
    bytes: [MaybeUninit<u8>; MAX_STR_SIZE],
    len: u32,
}

impl Debug for ConstStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstStr")
            .field("str", &self.as_str())
            .finish()
    }
}

#[cfg(feature = "serde")]
mod serde_bytes {
    use serde::{Deserialize, Serialize, Serializer};

    use crate::ConstStr;

    impl Serialize for ConstStr {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.as_str())
        }
    }

    impl<'de> Deserialize<'de> for ConstStr {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Ok(ConstStr::new(&s))
        }
    }
}

unsafe impl SerializeConst for ConstStr {
    const MEMORY_LAYOUT: Layout = Layout::List(ListLayout::new(
        std::mem::size_of::<Self>(),
        std::mem::offset_of!(Self, len),
        PrimitiveLayout {
            size: std::mem::size_of::<u32>(),
        },
        std::mem::offset_of!(Self, bytes),
        ArrayLayout {
            len: MAX_STR_SIZE,
            item_layout: &Layout::Primitive(PrimitiveLayout {
                size: std::mem::size_of::<u8>(),
            }),
        },
    ));
}

#[cfg(feature = "const-serialize-07")]
unsafe impl const_serialize_07::SerializeConst for ConstStr {
    const MEMORY_LAYOUT: const_serialize_07::Layout =
        const_serialize_07::Layout::Struct(const_serialize_07::StructLayout::new(
            std::mem::size_of::<Self>(),
            &[
                const_serialize_07::StructFieldLayout::new(
                    std::mem::offset_of!(Self, bytes),
                    const_serialize_07::Layout::List(const_serialize_07::ListLayout::new(
                        MAX_STR_SIZE,
                        &const_serialize_07::Layout::Primitive(
                            const_serialize_07::PrimitiveLayout::new(std::mem::size_of::<u8>()),
                        ),
                    )),
                ),
                const_serialize_07::StructFieldLayout::new(
                    std::mem::offset_of!(Self, len),
                    const_serialize_07::Layout::Primitive(
                        const_serialize_07::PrimitiveLayout::new(std::mem::size_of::<u32>()),
                    ),
                ),
            ],
        ));
}

impl ConstStr {
    /// Create a new constant string
    pub const fn new(s: &str) -> Self {
        let str_bytes = s.as_bytes();
        // This is serialized as a constant sized array in const-serialize-07 which requires all memory to be initialized
        let mut bytes = if cfg!(feature = "const-serialize-07") {
            [MaybeUninit::new(0); MAX_STR_SIZE]
        } else {
            [MaybeUninit::uninit(); MAX_STR_SIZE]
        };
        let mut i = 0;
        while i < str_bytes.len() {
            bytes[i] = MaybeUninit::new(str_bytes[i]);
            i += 1;
        }
        Self {
            bytes,
            len: str_bytes.len() as u32,
        }
    }

    /// Get the bytes of the initialized portion of the string
    const fn bytes(&self) -> &[u8] {
        // Safety: All bytes up to the pointer are initialized
        unsafe {
            &*(self.bytes.split_at(self.len as usize).0 as *const [MaybeUninit<u8>]
                as *const [u8])
        }
    }

    /// Get a reference to the string
    pub const fn as_str(&self) -> &str {
        let str_bytes = self.bytes();
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
            bytes[len as usize + i] = MaybeUninit::new(str_bytes[i]);
            i += 1;
        }
        Self {
            bytes,
            len: new_len as u32,
        }
    }

    /// Split the string at a byte index. The byte index must be a char boundary
    pub const fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.bytes().split_at(index);
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

impl PartialEq for ConstStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for ConstStr {}

impl PartialOrd for ConstStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConstStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for ConstStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
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
            bytes[0] = ((code >> 6) & 0x1F) as u8 | BYTE_CHAR_BOUNDARIES[1];
            bytes[1] = (code & 0x3F) as u8 | CONTINUED_CHAR_MASK;
        }
        3 => {
            bytes[0] = ((code >> 12) & 0x0F) as u8 | BYTE_CHAR_BOUNDARIES[2];
            bytes[1] = ((code >> 6) & 0x3F) as u8 | CONTINUED_CHAR_MASK;
            bytes[2] = (code & 0x3F) as u8 | CONTINUED_CHAR_MASK;
        }
        4 => {
            bytes[0] = ((code >> 18) & 0x07) as u8 | BYTE_CHAR_BOUNDARIES[3];
            bytes[1] = ((code >> 12) & 0x3F) as u8 | CONTINUED_CHAR_MASK;
            bytes[2] = ((code >> 6) & 0x3F) as u8 | CONTINUED_CHAR_MASK;
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
