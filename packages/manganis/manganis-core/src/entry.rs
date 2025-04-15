use const_serialize::SerializeConst;

use crate::BundledAsset;

#[derive(
    Debug,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[repr(C, u8)]
#[non_exhaustive]
pub enum ManganisEntry {
    /// An asset (ie a file) that should be copied by the bundler with some options.
    Asset(BundledAsset),

    /// A function that exports some additional metadata
    ///
    /// The function will end up in its own section
    Metadata(),
}
