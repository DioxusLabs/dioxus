use std::fmt::Debug;

use const_vec::ConstVec;

const CHUNK_SIZE: usize = core::mem::size_of::<u64>() * 4;
const PRIME_1: u64 = 0x9E3779B185EBCA87;
const PRIME_2: u64 = 0xC2B2AE3D27D4EB4F;
const PRIME_3: u64 = 0x165667B19E3779F9;
const PRIME_4: u64 = 0x85EBCA77C2B2AE63;
const PRIME_5: u64 = 0x27D4EB2F165667C5;

/// A byte range for one interned template string.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateStringSpan {
    off: u16,
    len: u16,
}

/// Const string interner used while building a template.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct StringInterner<const CAP: usize> {
    blob: ConstVec<u8, CAP>,
    spans: ConstVec<TemplateStringSpan, CAP>,
}

/// Static string interner stored on a lowered template.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StaticStringInterner {
    blob: &'static [u8],
    spans: &'static [TemplateStringSpan],
}

pub(crate) struct RuntimeStringInterner {
    blob: Vec<u8>,
    spans: Vec<TemplateStringSpan>,
}

impl<const CAP: usize> Debug for StringInterner<CAP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringInterner")
            .field("values", &self.values())
            .finish()
    }
}

impl Debug for StaticStringInterner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticStringInterner")
            .field("values", &self.values())
            .finish()
    }
}

impl<const CAP: usize> StringInterner<CAP> {
    /// Create an empty string interner.
    pub const fn new() -> Self {
        Self {
            blob: ConstVec::new_with_max_size(),
            spans: ConstVec::new_with_max_size(),
        }
    }

    /// Build an interner from a static string pool.
    ///
    /// This is intended for hand-written templates whose op tape already contains
    /// string indexes. Duplicate strings would change those indexes, so they panic.
    pub const fn from_unique_static_strings(strings: &[&str]) -> Self {
        let mut interner = Self::new();
        let mut index = 0;
        while index < strings.len() {
            let (next, id) = interner.intern(strings[index]);
            if id as usize != index {
                panic!("static template string pool contains duplicate strings");
            }
            interner = next;
            index += 1;
        }
        interner
    }

    /// Intern a string and return the updated interner with its string id.
    pub const fn intern(mut self, s: &str) -> (Self, u16) {
        let bytes = s.as_bytes();
        let mut span_index = 0;
        while span_index < self.spans.len() {
            let span = self.spans.at(span_index);
            if span.len as usize == bytes.len() {
                let mut byte_index = 0;
                let mut eq = true;
                while byte_index < bytes.len() {
                    if self.blob.at(span.off as usize + byte_index) != bytes[byte_index] {
                        eq = false;
                        break;
                    }
                    byte_index += 1;
                }
                if eq {
                    return (self, span_index as u16);
                }
            }
            span_index += 1;
        }

        let off = self.blob.len();
        if off > u16::MAX as usize || bytes.len() > u16::MAX as usize {
            panic!("template string capacity exceeded");
        }

        let mut byte_index = 0;
        while byte_index < bytes.len() {
            self.blob = self.blob.push(bytes[byte_index]);
            byte_index += 1;
        }

        let id = self.spans.len();
        self.spans = self.spans.push(TemplateStringSpan {
            off: off as u16,
            len: bytes.len() as u16,
        });
        (self, id as u16)
    }

    /// Borrow this interner as static template string storage.
    pub const fn as_static(&'static self) -> StaticStringInterner {
        StaticStringInterner {
            blob: self.blob.as_slice(),
            spans: self.spans.as_slice(),
        }
    }

    fn str_at(&self, id: u16) -> &str {
        let span = self.spans.as_slice()[id as usize];
        let start = span.off as usize;
        let end = start + span.len as usize;
        core::str::from_utf8(&self.blob.as_slice()[start..end]).unwrap()
    }

    fn values(&self) -> Vec<&str> {
        (0..self.spans.len())
            .map(|id| self.str_at(id as u16))
            .collect()
    }
}

impl Default for StaticStringInterner {
    fn default() -> Self {
        Self::empty()
    }
}

impl StaticStringInterner {
    /// Create an empty static string interner.
    pub const fn empty() -> Self {
        Self {
            blob: &[],
            spans: &[],
        }
    }

    /// Return the interned byte blob.
    pub const fn blob(&self) -> &'static [u8] {
        self.blob
    }

    /// Return the interned string spans.
    pub const fn spans(&self) -> &'static [TemplateStringSpan] {
        self.spans
    }

    /// Return the number of interned strings.
    pub const fn len(&self) -> usize {
        self.spans.len()
    }

    /// Return true if there are no interned strings.
    pub const fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Return one interned string by id.
    pub fn str_at(&self, id: u16) -> &'static str {
        let span = self.spans[id as usize];
        let start = span.off as usize;
        let end = start + span.len as usize;
        let blob: &'static [u8] = self.blob;
        // The blob is built only from valid Rust strings.
        unsafe { core::str::from_utf8_unchecked(&blob[start..end]) }
    }

    pub(crate) const fn hash_at(&self, id: u16, seed: u64) -> u64 {
        let span = self.spans[id as usize];
        xxh64_range(self.blob, span.off as usize, span.len as usize, seed)
    }

    fn values(&self) -> Vec<&'static str> {
        (0..self.spans.len())
            .map(|id| self.str_at(id as u16))
            .collect()
    }
}

impl RuntimeStringInterner {
    pub(crate) fn new() -> Self {
        Self {
            blob: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub(crate) fn intern(&mut self, s: &str) -> u16 {
        let bytes = s.as_bytes();
        for (id, span) in self.spans.iter().copied().enumerate() {
            let start = span.off as usize;
            let end = start + span.len as usize;
            if &self.blob[start..end] == bytes {
                return id as u16;
            }
        }

        let off = self.blob.len();
        if off > u16::MAX as usize || bytes.len() > u16::MAX as usize {
            panic!("template string capacity exceeded");
        }
        let id = self.spans.len();
        self.blob.extend_from_slice(bytes);
        self.spans.push(TemplateStringSpan {
            off: off as u16,
            len: bytes.len() as u16,
        });
        id as u16
    }

    pub(crate) fn leak(self) -> StaticStringInterner {
        StaticStringInterner {
            blob: Box::leak(self.blob.into_boxed_slice()),
            spans: Box::leak(self.spans.into_boxed_slice()),
        }
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for StaticStringInterner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for id in 0..self.len() {
            seq.serialize_element(self.str_at(id as u16))?;
        }
        seq.end()
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for StaticStringInterner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let strings = <Vec<String> as serde::Deserialize>::deserialize(deserializer)?;
        let mut interner = RuntimeStringInterner::new();
        for string in strings {
            interner.intern(&string);
        }
        Ok(interner.leak())
    }
}

const fn read_u32(input: &[u8], cursor: usize) -> u32 {
    input[cursor] as u32
        | (input[cursor + 1] as u32) << 8
        | (input[cursor + 2] as u32) << 16
        | (input[cursor + 3] as u32) << 24
}

const fn read_u64(input: &[u8], cursor: usize) -> u64 {
    input[cursor] as u64
        | (input[cursor + 1] as u64) << 8
        | (input[cursor + 2] as u64) << 16
        | (input[cursor + 3] as u64) << 24
        | (input[cursor + 4] as u64) << 32
        | (input[cursor + 5] as u64) << 40
        | (input[cursor + 6] as u64) << 48
        | (input[cursor + 7] as u64) << 56
}

const fn round(acc: u64, input: u64) -> u64 {
    acc.wrapping_add(input.wrapping_mul(PRIME_2))
        .rotate_left(31)
        .wrapping_mul(PRIME_1)
}

const fn merge_round(mut acc: u64, val: u64) -> u64 {
    acc ^= round(0, val);
    acc.wrapping_mul(PRIME_1).wrapping_add(PRIME_4)
}

const fn avalanche(mut input: u64) -> u64 {
    input ^= input >> 33;
    input = input.wrapping_mul(PRIME_2);
    input ^= input >> 29;
    input = input.wrapping_mul(PRIME_3);
    input ^= input >> 32;
    input
}

const fn finalize(mut input: u64, data: &[u8], mut cursor: usize, end: usize) -> u64 {
    let mut len = end - cursor;

    while len >= 8 {
        input ^= round(0, read_u64(data, cursor));
        cursor += core::mem::size_of::<u64>();
        len -= core::mem::size_of::<u64>();
        input = input
            .rotate_left(27)
            .wrapping_mul(PRIME_1)
            .wrapping_add(PRIME_4);
    }

    if len >= 4 {
        input ^= (read_u32(data, cursor) as u64).wrapping_mul(PRIME_1);
        cursor += core::mem::size_of::<u32>();
        len -= core::mem::size_of::<u32>();
        input = input
            .rotate_left(23)
            .wrapping_mul(PRIME_2)
            .wrapping_add(PRIME_3);
    }

    while len > 0 {
        input ^= (data[cursor] as u64).wrapping_mul(PRIME_5);
        cursor += core::mem::size_of::<u8>();
        len -= core::mem::size_of::<u8>();
        input = input.rotate_left(11).wrapping_mul(PRIME_1);
    }

    avalanche(input)
}

const fn xxh64_range(input: &[u8], offset: usize, len: usize, seed: u64) -> u64 {
    let input_len = len as u64;
    let mut cursor = offset;
    let end = offset + len;
    let mut result;

    if len >= CHUNK_SIZE {
        let mut v1 = seed.wrapping_add(PRIME_1).wrapping_add(PRIME_2);
        let mut v2 = seed.wrapping_add(PRIME_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(PRIME_1);

        loop {
            v1 = round(v1, read_u64(input, cursor));
            cursor += core::mem::size_of::<u64>();
            v2 = round(v2, read_u64(input, cursor));
            cursor += core::mem::size_of::<u64>();
            v3 = round(v3, read_u64(input, cursor));
            cursor += core::mem::size_of::<u64>();
            v4 = round(v4, read_u64(input, cursor));
            cursor += core::mem::size_of::<u64>();

            if end - cursor < CHUNK_SIZE {
                break;
            }
        }

        result = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));

        result = merge_round(result, v1);
        result = merge_round(result, v2);
        result = merge_round(result, v3);
        result = merge_round(result, v4);
    } else {
        result = seed.wrapping_add(PRIME_5);
    }

    result = result.wrapping_add(input_len);
    finalize(result, input, cursor, end)
}

#[cfg(test)]
mod tests {
    use super::StringInterner;

    #[test]
    fn deduplicates_strings() {
        const INTERNED: (StringInterner<16>, u16, u16) = {
            let (interner, first) = StringInterner::new().intern("div");
            let (interner, second) = interner.intern("div");
            (interner, first, second)
        };

        assert_eq!(INTERNED.1, INTERNED.2);
        assert_eq!(INTERNED.0.str_at(INTERNED.1), "div");
    }

    #[test]
    fn stores_distinct_strings() {
        const INTERNED: (StringInterner<16>, u16, u16) = {
            let (interner, first) = StringInterner::new().intern("div");
            let (interner, second) = interner.intern("span");
            (interner, first, second)
        };

        assert_ne!(INTERNED.1, INTERNED.2);
        assert_eq!(INTERNED.0.str_at(INTERNED.1), "div");
        assert_eq!(INTERNED.0.str_at(INTERNED.2), "span");
    }
}
