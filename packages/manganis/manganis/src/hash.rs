use const_serialize::{serialize_const, ConstVec, SerializeConst};

// From rustchash -  https://github.com/rust-lang/rustc-hash/blob/6745258da00b7251bed4a8461871522d0231a9c7/src/lib.rs#L98
const K: u64 = 0xf1357aea2e62a9c5;

pub(crate) struct ConstHasher {
    hash: u64,
}

impl ConstHasher {
    pub const fn new() -> Self {
        Self { hash: 0 }
    }

    pub const fn finish(&self) -> u64 {
        self.hash
    }

    pub const fn write(mut self, bytes: &[u8]) -> Self {
        let mut i = 0;
        while i < bytes.len() {
            self = self.write_byte(bytes[i]);
            i += 1;
        }
        self
    }

    pub const fn write_byte(mut self, byte: u8) -> Self {
        self.hash = self.hash.wrapping_add(byte as u64).wrapping_mul(K);
        self
    }

    pub const fn hash_by_bytes<T: SerializeConst>(self, item: &T) -> Self {
        let mut bytes = ConstVec::new();
        bytes = serialize_const(item, bytes);
        let bytes = bytes.as_ref();
        self.write(bytes)
    }
}
