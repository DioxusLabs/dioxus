use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// The options for a file asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub enum FileOptions {
    /// An image asset
    Image(ImageOptions),
    /// A video asset
    Video(VideoOptions),
    /// A font asset
    Font(FontOptions),
    /// A css asset
    Css(CssOptions),
    /// A JavaScript asset
    Js(JsOptions),
    /// A Json asset
    Json(JsonOptions),
    /// Any other asset
    Other(UnknownFileOptions),
}

impl Display for FileOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image(options) => write!(f, "{}", options),
            Self::Video(options) => write!(f, "{}", options),
            Self::Font(options) => write!(f, "{}", options),
            Self::Css(options) => write!(f, "{}", options),
            Self::Js(options) => write!(f, "{}", options),
            Self::Json(options) => write!(f, "{}", options),
            Self::Other(options) => write!(f, "{}", options),
        }
    }
}

impl FileOptions {
    /// Returns the default options for a given extension
    pub fn default_for_extension(extension: Option<&str>) -> Self {
        if let Some(extension) = extension {
            if extension == CssOptions::EXTENSION {
                return Self::Css(CssOptions::default());
            } else if extension == JsonOptions::EXTENSION {
                return Self::Json(JsonOptions::default());
            } else if let Ok(ty) = extension.parse::<ImageType>() {
                return Self::Image(ImageOptions::new(ty, None));
            } else if let Ok(ty) = extension.parse::<VideoType>() {
                return Self::Video(VideoOptions::new(ty));
            } else if let Ok(ty) = extension.parse::<FontType>() {
                return Self::Font(FontOptions::new(ty));
            } else if let Ok(ty) = extension.parse::<JsType>() {
                return Self::Js(JsOptions::new(ty));
            }
        }
        Self::Other(UnknownFileOptions {
            extension: extension.map(String::from),
        })
    }

    /// Returns the extension for this file
    pub fn extension(&self) -> Option<&str> {
        match self {
            Self::Image(options) => Some(options.ty.extension()),
            Self::Video(options) => Some(options.ty.extension()),
            Self::Font(options) => Some(options.ty.extension()),
            Self::Css(_) => Some(CssOptions::EXTENSION),
            Self::Js(js) => Some(js.ty.extension()),
            Self::Json(_) => Some(JsonOptions::EXTENSION),
            Self::Other(extension) => extension.extension.as_deref(),
        }
    }
}

impl Default for FileOptions {
    fn default() -> Self {
        Self::Other(UnknownFileOptions { extension: None })
    }
}

/// The options for an image asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub struct ImageOptions {
    compress: bool,
    size: Option<(u32, u32)>,
    preload: bool,
    ty: ImageType,
}

impl Display for ImageOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((x, y)) = self.size {
            write!(f, "{} ({}x{})", self.ty, x, y)?;
        } else {
            write!(f, "{}", self.ty)?;
        }
        if self.compress {
            write!(f, " (compressed)")?;
        }
        if self.preload {
            write!(f, " (preload)")?;
        }
        Ok(())
    }
}

impl ImageOptions {
    /// Creates a new image options struct
    pub fn new(ty: ImageType, size: Option<(u32, u32)>) -> Self {
        Self {
            compress: true,
            size,
            ty,
            preload: false,
        }
    }

    /// Returns whether the image should be preloaded
    pub fn preload(&self) -> bool {
        self.preload
    }

    /// Sets whether the image should be preloaded
    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
    }

    /// Returns the image type
    pub fn ty(&self) -> &ImageType {
        &self.ty
    }

    /// Sets the image type
    pub fn set_ty(&mut self, ty: ImageType) {
        self.ty = ty;
    }

    /// Returns the size of the image
    pub fn size(&self) -> Option<(u32, u32)> {
        self.size
    }

    /// Sets the size of the image
    pub fn set_size(&mut self, size: Option<(u32, u32)>) {
        self.size = size;
    }

    /// Returns whether the image should be compressed
    pub fn compress(&self) -> bool {
        self.compress
    }

    /// Sets whether the image should be compressed
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }
}

/// The type of an image
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Eq)]
pub enum ImageType {
    /// A png image
    Png,
    /// A jpg image
    Jpg,
    /// An avif image
    Avif,
    /// A webp image
    Webp,
}

impl ImageType {
    /// Returns the extension for this image type
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Avif => "avif",
            Self::Webp => "webp",
        }
    }
}

impl Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl FromStr for ImageType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpg),
            "avif" => Ok(Self::Avif),
            "webp" => Ok(Self::Webp),
            _ => Err(()),
        }
    }
}

/// The options for a video asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub struct VideoOptions {
    /// Whether the video should be compressed
    compress: bool,
    /// Whether the video should be preloaded
    preload: bool,
    /// The type of the video
    ty: VideoType,
}

impl Display for VideoOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty)?;
        if self.compress {
            write!(f, " (compressed)")?;
        }
        if self.preload {
            write!(f, " (preload)")?;
        }
        Ok(())
    }
}

impl VideoOptions {
    /// Creates a new video options struct
    pub fn new(ty: VideoType) -> Self {
        Self {
            compress: true,
            ty,
            preload: false,
        }
    }

    /// Returns the type of the video
    pub fn ty(&self) -> &VideoType {
        &self.ty
    }

    /// Sets the type of the video
    pub fn set_ty(&mut self, ty: VideoType) {
        self.ty = ty;
    }

    /// Returns whether the video should be compressed
    pub fn compress(&self) -> bool {
        self.compress
    }

    /// Sets whether the video should be compressed
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }

    /// Returns whether the video should be preloaded
    pub fn preload(&self) -> bool {
        self.preload
    }

    /// Sets whether the video should be preloaded
    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
    }
}

/// The type of a video
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub enum VideoType {
    /// An mp4 video
    MP4,
    /// A webm video
    Webm,
    /// A gif video
    GIF,
}

impl VideoType {
    /// Returns the extension for this video type
    pub fn extension(&self) -> &'static str {
        match self {
            Self::MP4 => "mp4",
            Self::Webm => "webm",
            Self::GIF => "gif",
        }
    }
}

impl Display for VideoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl FromStr for VideoType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mp4" => Ok(Self::MP4),
            "webm" => Ok(Self::Webm),
            "gif" => Ok(Self::GIF),
            _ => Err(()),
        }
    }
}

/// The options for a font asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub struct FontOptions {
    ty: FontType,
}

impl Display for FontOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty)
    }
}

impl FontOptions {
    /// Creates a new font options struct
    pub fn new(ty: FontType) -> Self {
        Self { ty }
    }

    /// Returns the type of the font
    pub fn ty(&self) -> &FontType {
        &self.ty
    }
}

/// The type of a font
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub enum FontType {
    /// A ttf (TrueType) font
    TTF,
    /// A woff (Web Open Font Format) font
    WOFF,
    /// A woff2 (Web Open Font Format 2) font
    WOFF2,
}

impl FontType {
    /// Returns the extension for this font type
    pub fn extension(&self) -> &'static str {
        match self {
            Self::TTF => "ttf",
            Self::WOFF => "woff",
            Self::WOFF2 => "woff2",
        }
    }
}

impl FromStr for FontType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ttf" => Ok(Self::TTF),
            "woff" => Ok(Self::WOFF),
            "woff2" => Ok(Self::WOFF2),
            _ => Err(()),
        }
    }
}

impl Display for FontType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TTF => write!(f, "ttf"),
            Self::WOFF => write!(f, "woff"),
            Self::WOFF2 => write!(f, "woff2"),
        }
    }
}

/// The options for a css asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub struct CssOptions {
    minify: bool,
    preload: bool,
}

impl Default for CssOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for CssOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.minify {
            write!(f, "minified")?;
        }
        if self.preload {
            write!(f, " (preload)")?;
        }
        Ok(())
    }
}

impl CssOptions {
    const EXTENSION: &'static str = "css";

    /// Creates a new css options struct
    pub const fn new() -> Self {
        Self {
            minify: true,
            preload: false,
        }
    }

    /// Returns whether the css should be minified
    pub fn minify(&self) -> bool {
        self.minify
    }

    /// Sets whether the css should be minified
    pub fn set_minify(&mut self, minify: bool) {
        self.minify = minify;
    }

    /// Returns whether the css should be preloaded
    pub fn preload(&self) -> bool {
        self.preload
    }

    /// Sets whether the css should be preloaded
    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
    }
}

/// The type of a Javascript asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Default, Eq)]
pub enum JsType {
    /// A js asset
    #[default]
    Js,
    // TODO: support ts files
}

impl JsType {
    /// Returns the extension for this js type
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Js => "js",
        }
    }
}

impl FromStr for JsType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "js" => Ok(Self::Js),
            _ => Err(()),
        }
    }
}

impl Display for JsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

/// The options for a Javascript asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Default, Eq)]
pub struct JsOptions {
    ty: JsType,
    minify: bool,
    preload: bool,
}

impl Display for JsOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "js")?;
        Ok(())
    }
}

impl JsOptions {
    /// Creates a new js options struct
    pub fn new(ty: JsType) -> Self {
        Self {
            ty,
            preload: false,
            minify: true,
        }
    }

    /// Returns whether the js should be preloaded
    pub fn preload(&self) -> bool {
        self.preload
    }

    /// Sets whether the js should be preloaded
    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
    }

    /// Returns if the js should be minified
    pub fn minify(&self) -> bool {
        self.minify
    }

    /// Sets if the js should be minified
    pub fn set_minify(&mut self, minify: bool) {
        self.minify = minify;
    }
}

/// The options for a Json asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Default, Eq)]
pub struct JsonOptions {
    preload: bool,
}

impl Display for JsonOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "json")?;
        Ok(())
    }
}

impl JsonOptions {
    /// The extension of the json asset
    pub const EXTENSION: &'static str = "json";

    /// Creates a new json options struct
    pub fn new() -> Self {
        Self { preload: false }
    }

    /// Returns whether the json should be preloaded
    pub fn preload(&self) -> bool {
        self.preload
    }

    /// Sets whether the json should be preloaded
    pub fn set_preload(&mut self, preload: bool) {
        self.preload = preload;
    }
}

/// The options for an unknown file asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone, Hash, Eq)]
pub struct UnknownFileOptions {
    extension: Option<String>,
}

impl Display for UnknownFileOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(extension) = &self.extension {
            write!(f, "{}", extension)?;
        }
        Ok(())
    }
}

impl UnknownFileOptions {
    /// Creates a new unknown file options struct
    pub fn new(extension: Option<String>) -> Self {
        Self { extension }
    }

    /// Returns the extension of the file
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }
}
