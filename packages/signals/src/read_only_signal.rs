use crate::Read;

/// A signal that can only be read from.
pub type ReadOnlySignal<T> = Read<T>;

/// A signal that can only be read from.
pub type ReadSignal<T> = ReadOnlySignal<T>;
