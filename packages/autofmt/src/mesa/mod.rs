//! An implementation of Philip Karlton's Mesa pretty-printer algorithm, as
//! described in Derek Oppen's "Prettyprinting" (1980) and adapted by rustc's
//! `rustc_ast_pretty` and dtolnay's `prettyplease`.
//!
//! The printer consumes a stream of `Begin`/`End` box tokens, `Break` tokens,
//! and `String` tokens, and decides where to insert line breaks based on how
//! much of each box fits within the margin. Boxes are either "consistent"
//! (if any break inside fires, they all fire) or "inconsistent" (each break
//! fires only if the following content does not fit).
//!
//! `algorithm.rs`, `ring.rs` and `convenience.rs` are vendored from
//! prettyplease 0.2 (dual licensed Apache-2.0/MIT), which prettyplease in turn
//! adapted from rustc_ast_pretty. They are vendored because prettyplease does
//! not expose its printer publicly.

mod algorithm;
mod convenience;
mod ring;

pub use algorithm::{BreakToken, Printer};

/// The `blank_space` of a break that always fires (a hard break).
pub const SIZE_INFINITY_SPACE: usize = algorithm::SIZE_INFINITY as usize;

/// The maximum width of a line before the printer prefers to break it.
pub const MARGIN: isize = 80;

/// The unit of indentation, in columns.
pub const INDENT: isize = 4;

/// The minimum number of columns the printer will keep available for content
/// on a line, no matter how deeply indented the code is.
pub const MIN_SPACE: isize = 40;
