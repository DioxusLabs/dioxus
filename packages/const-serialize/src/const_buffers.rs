/// A buffer that can be read from at compile time. This is very similar to [Cursor](std::io::Cursor) but is
/// designed to be used in const contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstReadBuffer<'a> {
    location: usize,
    memory: &'a [u8],
}

impl<'a> ConstReadBuffer<'a> {
    /// Create a new buffer from a byte slice
    pub const fn new(memory: &'a [u8]) -> Self {
        Self {
            location: 0,
            memory,
        }
    }

    /// Get the next byte from the buffer. Returns `None` if the buffer is empty.
    /// This will return the new version of the buffer with the first byte removed.
    pub const fn get(mut self) -> Option<(Self, u8)> {
        if self.location >= self.memory.len() {
            return None;
        }
        let value = self.memory[self.location];
        self.location += 1;
        Some((self, value))
    }

    /// Get a reference to the underlying byte slice
    pub const fn as_ref(&self) -> &[u8] {
        self.memory
    }
}
