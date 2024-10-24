/// A builder for a font asset. This must be used in the `asset!` macro.
///
/// > **Note**: This will do nothing outside of the `asset!` macro
pub struct FontAssetBuilder;

impl FontAssetBuilder {
    /// Sets the font family of the font
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(font().families(["Roboto"]));
    /// ```
    #[allow(unused)]
    pub const fn families<const N: usize>(self, families: [&'static str; N]) -> Self {
        Self
    }

    /// Sets the font weight of the font
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]));
    /// ```
    #[allow(unused)]
    pub const fn weights<const N: usize>(self, weights: [u32; N]) -> Self {
        Self
    }

    /// Sets the subset of text that the font needs to support. The font will only include the characters in the text which can make the font file size significantly smaller
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]).text("Hello, world!"));
    /// ```
    #[allow(unused)]
    pub const fn text(self, text: &'static str) -> Self {
        Self
    }

    /// Sets the [display](https://www.w3.org/TR/css-fonts-4/#font-display-desc) of the font. The display control what happens when the font is unavailable
    ///
    /// > **Note**: This will do nothing outside of the `asset!` macro
    ///
    /// ```rust
    /// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]).text("Hello, world!").display("swap"));
    /// ```
    #[allow(unused)]
    pub const fn display(self, display: &'static str) -> Self {
        Self
    }
}

/// Create a font asset
///
/// > **Note**: This will do nothing outside of the `asset!` macro
///
/// You can use the font builder to collect fonts that will be included in the final binary from google fonts
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]));
/// ```
/// You can specify weights for the fonts
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]));
/// ```
/// Or set the text to only include the characters you need
/// ```rust
/// const _: &str = manganis::asset!(font().families(["Roboto"]).weights([200]).text("Hello, world!"));
/// ```
#[allow(unused)]
pub const fn font() -> FontAssetBuilder {
    FontAssetBuilder
}
