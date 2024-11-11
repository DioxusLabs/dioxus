use crate::const_vec::ConstVec;

pub struct ConstReadBuffer<'a> {
    location: usize,
    memory: &'a [u8],
}

impl<'a> ConstReadBuffer<'a> {
    pub const fn new(memory: &'a [u8]) -> Self {
        Self {
            location: 0,
            memory,
        }
    }

    pub const fn get(mut self) -> Option<(Self, u8)> {
        if self.location >= self.memory.len() {
            return None;
        }
        let value = self.memory[self.location];
        self.location += 1;
        Some((self, value))
    }

    pub const fn as_ref(&self) -> &[u8] {
        self.memory
    }
}

pub struct ConstWriteBuffer {
    memory: ConstVec<u8>,
}

impl Default for ConstWriteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstWriteBuffer {
    pub const fn new() -> Self {
        Self {
            memory: ConstVec::new(),
        }
    }

    pub const fn from(vec: ConstVec<u8>) -> Self {
        Self { memory: vec }
    }

    pub const fn push(self, value: u8) -> Self {
        let memory = self.memory.push(value);
        Self { memory }
    }

    pub const fn as_ref(&self) -> &[u8] {
        self.memory.as_ref()
    }

    /// Get the underlying const vec for this buffer
    pub const fn inner(self) -> ConstVec<u8> {
        self.memory
    }

    pub const fn read(&self) -> ConstReadBuffer {
        ConstReadBuffer::new(self.memory.as_ref())
    }
}
