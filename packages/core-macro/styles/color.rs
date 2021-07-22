use std::fmt;

/// A color that possibly is possibly code, rather than a literal
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicColor {
    Literal(Color),
    /// The type of the block is not checked here (it is checked by typeck).
    Dynamic(syn::Block),
}

impl DynamicColor {
    pub fn is_dynamic(&self) -> bool {
        match self {
            DynamicColor::Dynamic(_) => true,
            DynamicColor::Literal(_) => false,
        }
    }
}

impl fmt::Display for DynamicColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DynamicColor::Dynamic(_) => Ok(()),
            DynamicColor::Literal(color) => color.fmt(f),
        }
    }
}

// TODO other color variants.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Color {
    HexRGB(u8, u8, u8),
    HexRGBA(u8, u8, u8, u8),
    // Invariants: `0 <= .0 < 360`, `0 <= .1 < 100`, `0 <= .2 < 100`.
    HSL(f64, f64, f64),
    // Invariants: `0 <= .0 < 360`, `0 <= .1 < 100`, `0 <= .2 < 100`, `0 <= .3 < 1`.
    HSLA(f64, f64, f64, f64),

    // Red HTML Color Names
    /// rgb(205, 92, 92)
    IndianRed,
    /// rgb(240, 128, 128)
    LightCoral,
    /// rgb(250, 128, 114)
    Salmon,
    /// rgb(233, 150, 122)
    DarkSalmon,
    /// rgb(255, 160, 122)
    LightSalmon,
    /// rgb(220, 20, 60)
    Crimson,
    /// rgb(255, 0, 0)
    Red,
    /// rgb(178, 34, 34)
    FireBrick,
    /// rgb(139, 0, 0)
    DarkRed,
    // Pink HTML Color Names
    /// rgb(255, 192, 203)
    Pink,
    /// rgb(255, 182, 193)
    LightPink,
    /// rgb(255, 105, 180)
    HotPink,
    /// rgb(255, 20, 147)
    DeepPink,
    /// rgb(199, 21, 133)
    MediumVioletRed,
    /// rgb(219, 112, 147)
    PaleVioletRed,
    //Orange HTML Color Names
    // /// rgb(255, 160, 122) redefined
    // LightSalmon,
    /// rgb(255, 127, 80)
    Coral,
    /// rgb(255, 99, 71)
    Tomato,
    /// rgb(255, 69, 0)
    OrangeRed,
    /// rgb(255, 140, 0)
    DarkOrange,
    /// rgb(255, 165, 0)
    Orange,
    // Yellow HTML Color Names
    /// rgb(255, 215, 0)
    Gold,
    /// rgb(255, 255, 0)
    Yellow,
    /// rgb(255, 255, 224)
    LightYellow,
    /// rgb(255, 250, 205)
    LemonChiffon,
    /// rgb(250, 250, 210)
    LightGoldenrodYellow,
    /// rgb(255, 239, 213)
    PapayaWhip,
    /// rgb(255, 228, 181)
    Moccasin,
    /// rgb(255, 218, 185)
    PeachPuff,
    /// rgb(238, 232, 170)
    PaleGoldenrod,
    /// rgb(240, 230, 140)
    Khaki,
    /// rgb(189, 183, 107)
    DarkKhaki,
    // Purple HTML Color Names
    /// rgb(230, 230, 250)
    Lavender,
    /// rgb(216, 191, 216)
    Thistle,
    /// rgb(221, 160, 221)
    Plum,
    /// rgb(238, 130, 238)
    Violet,
    /// rgb(218, 112, 214)
    Orchid,
    /// rgb(255, 0, 255)
    Fuchsia,
    /// rgb(255, 0, 255)
    Magenta,
    /// rgb(186, 85, 211)
    MediumOrchid,
    /// rgb(147, 112, 219)
    MediumPurple,
    /// rgb(102, 51, 153)
    RebeccaPurple,
    /// rgb(138, 43, 226)
    BlueViolet,
    /// rgb(148, 0, 211)
    DarkViolet,
    /// rgb(153, 50, 204)
    DarkOrchid,
    /// rgb(139, 0, 139)
    DarkMagenta,
    /// rgb(128, 0, 128)
    Purple,
    /// rgb(75, 0, 130)
    Indigo,
    /// rgb(106, 90, 205)
    SlateBlue,
    /// rgb(72, 61, 139)
    DarkSlateBlue,
    /// rgb(123, 104, 238)
    MediumSlateBlue,
    // Green HTML Color Names
    /// rgb(173, 255, 47)
    GreenYellow,
    /// rgb(127, 255, 0)
    Chartreuse,
    /// rgb(124, 252, 0)
    LawnGreen,
    /// rgb(0, 255, 0)
    Lime,
    /// rgb(50, 205, 50)
    LimeGreen,
    /// rgb(152, 251, 152)
    PaleGreen,
    /// rgb(144, 238, 144)
    LightGreen,
    /// rgb(0, 250, 154)
    MediumSpringGreen,
    /// rgb(0, 255, 127)
    SpringGreen,
    /// rgb(60, 179, 113)
    MediumSeaGreen,
    /// rgb(46, 139, 87)
    SeaGreen,
    /// rgb(34, 139, 34)
    ForestGreen,
    /// rgb(0, 128, 0)
    Green,
    /// rgb(0, 100, 0)
    DarkGreen,
    /// rgb(154, 205, 50)
    YellowGreen,
    /// rgb(107, 142, 35)
    OliveDrab,
    /// rgb(128, 128, 0)
    Olive,
    /// rgb(85, 107, 47)
    DarkOliveGreen,
    /// rgb(102, 205, 170)
    MediumAquamarine,
    /// rgb(143, 188, 139)
    DarkSeaGreen,
    /// rgb(32, 178, 170)
    LightSeaGreen,
    /// rgb(0, 139, 139)
    DarkCyan,
    /// rgb(0, 128, 128)
    Teal,
    // Blue HTML Color Names
    /// rgb(0, 255, 255)
    Aqua,
    /// rgb(0, 255, 255)
    Cyan,
    /// rgb(224, 255, 255)
    LightCyan,
    /// rgb(175, 238, 238)
    PaleTurquoise,
    /// rgb(127, 255, 212)
    Aquamarine,
    /// rgb(64, 224, 208)
    Turquoise,
    /// rgb(72, 209, 204)
    MediumTurquoise,
    /// rgb(0, 206, 209)
    DarkTurquoise,
    /// rgb(95, 158, 160)
    CadetBlue,
    /// rgb(70, 130, 180)
    SteelBlue,
    /// rgb(176, 196, 222)
    LightSteelBlue,
    /// rgb(176, 224, 230)
    PowderBlue,
    /// rgb(173, 216, 230)
    LightBlue,
    /// rgb(135, 206, 235)
    SkyBlue,
    /// rgb(135, 206, 250)
    LightSkyBlue,
    /// rgb(0, 191, 255)
    DeepSkyBlue,
    /// rgb(30, 144, 255)
    DodgerBlue,
    /// rgb(100, 149, 237)
    CornflowerBlue,
    // /// rgb(123, 104, 238) duplicate
    //MediumSlateBlue,
    /// rgb(65, 105, 225)
    RoyalBlue,
    /// rgb(0, 0, 255)
    Blue,
    /// rgb(0, 0, 205)
    MediumBlue,
    /// rgb(0, 0, 139)
    DarkBlue,
    /// rgb(0, 0, 128)
    Navy,
    /// rgb(25, 25, 112)
    MidnightBlue,
    // Brown HTML Color Names
    /// rgb(255, 248, 220)
    Cornsilk,
    /// rgb(255, 235, 205)
    BlanchedAlmond,
    /// rgb(255, 228, 196)
    Bisque,
    /// rgb(255, 222, 173)
    NavajoWhite,
    /// rgb(245, 222, 179)
    Wheat,
    /// rgb(222, 184, 135)
    BurlyWood,
    /// rgb(210, 180, 140)
    Tan,
    /// rgb(188, 143, 143)
    RosyBrown,
    /// rgb(244, 164, 96)
    SandyBrown,
    /// rgb(218, 165, 32)
    Goldenrod,
    /// rgb(184, 134, 11)
    DarkGoldenrod,
    /// rgb(205, 133, 63)
    Peru,
    /// rgb(210, 105, 30)
    Chocolate,
    /// rgb(139, 69, 19)
    SaddleBrown,
    /// rgb(160, 82, 45)
    Sienna,
    /// rgb(165, 42, 42)
    Brown,
    /// rgb(128, 0, 0)
    Maroon,
    // White HTML Color Names
    /// rgb(255, 255, 255)
    White,
    /// rgb(255, 250, 250)
    Snow,
    /// rgb(240, 255, 240)
    HoneyDew,
    /// rgb(245, 255, 250)
    MintCream,
    /// rgb(240, 255, 255)
    Azure,
    /// rgb(240, 248, 255)
    AliceBlue,
    /// rgb(248, 248, 255)
    GhostWhite,
    /// rgb(245, 245, 245)
    WhiteSmoke,
    /// rgb(255, 245, 238)
    SeaShell,
    /// rgb(245, 245, 220)
    Beige,
    /// rgb(253, 245, 230)
    OldLace,
    /// rgb(255, 250, 240)
    FloralWhite,
    /// rgb(255, 255, 240)
    Ivory,
    /// rgb(250, 235, 215)
    AntiqueWhite,
    /// rgb(250, 240, 230)
    Linen,
    /// rgb(255, 240, 245)
    LavenderBlush,
    /// rgb(255, 228, 225)
    MistyRose,
    // Gray HTML Color Names
    /// rgb(220, 220, 220)
    Gainsboro,
    /// rgb(211, 211, 211)
    LightGray,
    /// rgb(192, 192, 192)
    Silver,
    /// rgb(169, 169, 169)
    DarkGray,
    /// rgb(128, 128, 128)
    Gray,
    /// rgb(105, 105, 105)
    DimGray,
    /// rgb(119, 136, 153)
    LightSlateGray,
    /// rgb(112, 128, 144)
    SlateGray,
    /// rgb(47, 79, 79)
    DarkSlateGray,
    /// rgb(0, 0, 0)
    Black,
}

impl Color {
    // todo similar for others
    pub fn to_rgb(self) -> Color {
        use Color::*;
        match self {
            HexRGB(r, g, b) => HexRGB(r, g, b),
            HexRGBA(r, g, b, _) => HexRGB(r, g, b),
            HSL(h, s, l) => {
                let s = s * 0.01; // percent conversion
                let l = l * 0.01; // percent conversion
                let (r, g, b) = hsl_to_rgb(h, s, l);
                HexRGB((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
            }
            HSLA(h, s, l, _) => Color::to_rgb(HSL(h, s, l)),
            IndianRed => HexRGB(205, 92, 92),
            LightCoral => HexRGB(240, 128, 128),
            Salmon => HexRGB(250, 128, 114),
            DarkSalmon => HexRGB(233, 150, 122),
            LightSalmon => HexRGB(255, 160, 122),
            Crimson => HexRGB(220, 20, 60),
            Red => HexRGB(255, 0, 0),
            FireBrick => HexRGB(178, 34, 34),
            DarkRed => HexRGB(139, 0, 0),
            Pink => HexRGB(255, 192, 203),
            LightPink => HexRGB(255, 182, 193),
            HotPink => HexRGB(255, 105, 180),
            DeepPink => HexRGB(255, 20, 147),
            MediumVioletRed => HexRGB(199, 21, 133),
            PaleVioletRed => HexRGB(219, 112, 147),
            Coral => HexRGB(255, 127, 80),
            Tomato => HexRGB(255, 99, 71),
            OrangeRed => HexRGB(255, 69, 0),
            DarkOrange => HexRGB(255, 140, 0),
            Orange => HexRGB(255, 165, 0),
            Gold => HexRGB(255, 215, 0),
            Yellow => HexRGB(255, 255, 0),
            LightYellow => HexRGB(255, 255, 224),
            LemonChiffon => HexRGB(255, 250, 205),
            LightGoldenrodYellow => HexRGB(250, 250, 210),
            PapayaWhip => HexRGB(255, 239, 213),
            Moccasin => HexRGB(255, 228, 181),
            PeachPuff => HexRGB(255, 218, 185),
            PaleGoldenrod => HexRGB(238, 232, 170),
            Khaki => HexRGB(240, 230, 140),
            DarkKhaki => HexRGB(189, 183, 107),
            Lavender => HexRGB(230, 230, 250),
            Thistle => HexRGB(216, 191, 216),
            Plum => HexRGB(221, 160, 221),
            Violet => HexRGB(238, 130, 238),
            Orchid => HexRGB(218, 112, 214),
            Fuchsia => HexRGB(255, 0, 255),
            Magenta => HexRGB(255, 0, 255),
            MediumOrchid => HexRGB(186, 85, 211),
            MediumPurple => HexRGB(147, 112, 219),
            RebeccaPurple => HexRGB(102, 51, 153),
            BlueViolet => HexRGB(138, 43, 226),
            DarkViolet => HexRGB(148, 0, 211),
            DarkOrchid => HexRGB(153, 50, 204),
            DarkMagenta => HexRGB(139, 0, 139),
            Purple => HexRGB(128, 0, 128),
            Indigo => HexRGB(75, 0, 130),
            SlateBlue => HexRGB(106, 90, 205),
            DarkSlateBlue => HexRGB(72, 61, 139),
            MediumSlateBlue => HexRGB(123, 104, 238),
            GreenYellow => HexRGB(173, 255, 47),
            Chartreuse => HexRGB(127, 255, 0),
            LawnGreen => HexRGB(124, 252, 0),
            Lime => HexRGB(0, 255, 0),
            LimeGreen => HexRGB(50, 205, 50),
            PaleGreen => HexRGB(152, 251, 152),
            LightGreen => HexRGB(144, 238, 144),
            MediumSpringGreen => HexRGB(0, 250, 154),
            SpringGreen => HexRGB(0, 255, 127),
            MediumSeaGreen => HexRGB(60, 179, 113),
            SeaGreen => HexRGB(46, 139, 87),
            ForestGreen => HexRGB(34, 139, 34),
            Green => HexRGB(0, 128, 0),
            DarkGreen => HexRGB(0, 100, 0),
            YellowGreen => HexRGB(154, 205, 50),
            OliveDrab => HexRGB(107, 142, 35),
            Olive => HexRGB(128, 128, 0),
            DarkOliveGreen => HexRGB(85, 107, 47),
            MediumAquamarine => HexRGB(102, 205, 170),
            DarkSeaGreen => HexRGB(143, 188, 139),
            LightSeaGreen => HexRGB(32, 178, 170),
            DarkCyan => HexRGB(0, 139, 139),
            Teal => HexRGB(0, 128, 128),
            Aqua => HexRGB(0, 255, 255),
            Cyan => HexRGB(0, 255, 255),
            LightCyan => HexRGB(224, 255, 255),
            PaleTurquoise => HexRGB(175, 238, 238),
            Aquamarine => HexRGB(127, 255, 212),
            Turquoise => HexRGB(64, 224, 208),
            MediumTurquoise => HexRGB(72, 209, 204),
            DarkTurquoise => HexRGB(0, 206, 209),
            CadetBlue => HexRGB(95, 158, 160),
            SteelBlue => HexRGB(70, 130, 180),
            LightSteelBlue => HexRGB(176, 196, 222),
            PowderBlue => HexRGB(176, 224, 230),
            LightBlue => HexRGB(173, 216, 230),
            SkyBlue => HexRGB(135, 206, 235),
            LightSkyBlue => HexRGB(135, 206, 250),
            DeepSkyBlue => HexRGB(0, 191, 255),
            DodgerBlue => HexRGB(30, 144, 255),
            CornflowerBlue => HexRGB(100, 149, 237),
            RoyalBlue => HexRGB(65, 105, 225),
            Blue => HexRGB(0, 0, 255),
            MediumBlue => HexRGB(0, 0, 205),
            DarkBlue => HexRGB(0, 0, 139),
            Navy => HexRGB(0, 0, 128),
            MidnightBlue => HexRGB(25, 25, 112),
            Cornsilk => HexRGB(255, 248, 220),
            BlanchedAlmond => HexRGB(255, 235, 205),
            Bisque => HexRGB(255, 228, 196),
            NavajoWhite => HexRGB(255, 222, 173),
            Wheat => HexRGB(245, 222, 179),
            BurlyWood => HexRGB(222, 184, 135),
            Tan => HexRGB(210, 180, 140),
            RosyBrown => HexRGB(188, 143, 143),
            SandyBrown => HexRGB(244, 164, 96),
            Goldenrod => HexRGB(218, 165, 32),
            DarkGoldenrod => HexRGB(184, 134, 11),
            Peru => HexRGB(205, 133, 63),
            Chocolate => HexRGB(210, 105, 30),
            SaddleBrown => HexRGB(139, 69, 19),
            Sienna => HexRGB(160, 82, 45),
            Brown => HexRGB(165, 42, 42),
            Maroon => HexRGB(128, 0, 0),
            White => HexRGB(255, 255, 255),
            Snow => HexRGB(255, 250, 250),
            HoneyDew => HexRGB(240, 255, 240),
            MintCream => HexRGB(245, 255, 250),
            Azure => HexRGB(240, 255, 255),
            AliceBlue => HexRGB(240, 248, 255),
            GhostWhite => HexRGB(248, 248, 255),
            WhiteSmoke => HexRGB(245, 245, 245),
            SeaShell => HexRGB(255, 245, 238),
            Beige => HexRGB(245, 245, 220),
            OldLace => HexRGB(253, 245, 230),
            FloralWhite => HexRGB(255, 250, 240),
            Ivory => HexRGB(255, 255, 240),
            AntiqueWhite => HexRGB(250, 235, 215),
            Linen => HexRGB(250, 240, 230),
            LavenderBlush => HexRGB(255, 240, 245),
            MistyRose => HexRGB(255, 228, 225),
            Gainsboro => HexRGB(220, 220, 220),
            LightGray => HexRGB(211, 211, 211),
            Silver => HexRGB(192, 192, 192),
            DarkGray => HexRGB(169, 169, 169),
            Gray => HexRGB(128, 128, 128),
            DimGray => HexRGB(105, 105, 105),
            LightSlateGray => HexRGB(119, 136, 153),
            SlateGray => HexRGB(112, 128, 144),
            DarkSlateGray => HexRGB(47, 79, 79),
            Black => HexRGB(0, 0, 0),
        }
    }

    pub fn from_named(name: &str) -> Option<Self> {
        // todo use a faster search (e.g. hashmap, aho-corasick)
        use Color::*;
        Some(match name {
            "indianred" => IndianRed,
            "lightcoral" => LightCoral,
            "salmon" => Salmon,
            "darksalmon" => DarkSalmon,
            "lightsalmon" => LightSalmon,
            "crimson" => Crimson,
            "red" => Red,
            "firebrick" => FireBrick,
            "darkred" => DarkRed,
            "pink" => Pink,
            "lightpink" => LightPink,
            "hotpink" => HotPink,
            "deeppink" => DeepPink,
            "mediumvioletred" => MediumVioletRed,
            "palevioletred" => PaleVioletRed,
            "coral" => Coral,
            "tomato" => Tomato,
            "orangered" => OrangeRed,
            "darkorange" => DarkOrange,
            "orange" => Orange,
            "gold" => Gold,
            "yellow" => Yellow,
            "lightyellow" => LightYellow,
            "lemonchiffon" => LemonChiffon,
            "lightgoldenrodyellow" => LightGoldenrodYellow,
            "papayawhip" => PapayaWhip,
            "Moccasin" => Moccasin,
            "Peachpuff" => PeachPuff,
            "palegoldenrod" => PaleGoldenrod,
            "khaki" => Khaki,
            "darkkhaki" => DarkKhaki,
            "lavender" => Lavender,
            "thistle" => Thistle,
            "plum" => Plum,
            "violet" => Violet,
            "orchid" => Orchid,
            "fuchsia" => Fuchsia,
            "magenta" => Magenta,
            "mediumorchid" => MediumOrchid,
            "mediumpurple" => MediumPurple,
            "rebeccapurple" => RebeccaPurple,
            "blueviolet" => BlueViolet,
            "darkviolet" => DarkViolet,
            "darkorchid" => DarkOrchid,
            "darkmagenta" => DarkMagenta,
            "purple" => Purple,
            "indigo" => Indigo,
            "slateblue" => SlateBlue,
            "darkslateblue" => DarkSlateBlue,
            "mediumslateblue" => MediumSlateBlue,
            "greenyellow" => GreenYellow,
            "chartreuse" => Chartreuse,
            "lawngreen" => LawnGreen,
            "lime" => Lime,
            "limegreen" => LimeGreen,
            "palegreen" => PaleGreen,
            "lightgreen" => LightGreen,
            "mediumspringgreen" => MediumSpringGreen,
            "springgreen" => SpringGreen,
            "mediumseagreen" => MediumSeaGreen,
            "seagreen" => SeaGreen,
            "forestgreen" => ForestGreen,
            "green" => Green,
            "darkgreen" => DarkGreen,
            "yellowgreen" => YellowGreen,
            "olivedrab" => OliveDrab,
            "olive" => Olive,
            "darkolivegreen" => DarkOliveGreen,
            "mediumaquamarine" => MediumAquamarine,
            "darkseagreen" => DarkSeaGreen,
            "lightseagreen" => LightSeaGreen,
            "darkcyan" => DarkCyan,
            "teal" => Teal,
            "aqua" => Aqua,
            "cyan" => Cyan,
            "lightcyan" => LightCyan,
            "paleturquoise" => PaleTurquoise,
            "aquamarine" => Aquamarine,
            "turquoise" => Turquoise,
            "mediumturquoise" => MediumTurquoise,
            "darkturquoise" => DarkTurquoise,
            "cadetblue" => CadetBlue,
            "steelblue" => SteelBlue,
            "lightsteelblue" => LightSteelBlue,
            "powderblue" => PowderBlue,
            "lightblue" => LightBlue,
            "skyblue" => SkyBlue,
            "lightskyblue" => LightSkyBlue,
            "deepskyblue" => DeepSkyBlue,
            "dodgerblue" => DodgerBlue,
            "cornflowerblue" => CornflowerBlue,
            "royalblue" => RoyalBlue,
            "blue" => Blue,
            "mediumblue" => MediumBlue,
            "darkblue" => DarkBlue,
            "navy" => Navy,
            "midnightblue" => MidnightBlue,
            "cornsilk" => Cornsilk,
            "blanchedalmond" => BlanchedAlmond,
            "bisque" => Bisque,
            "navajowhite" => NavajoWhite,
            "wheat" => Wheat,
            "burlywood" => BurlyWood,
            "tan" => Tan,
            "rosybrown" => RosyBrown,
            "sandybrown" => SandyBrown,
            "goldenrod" => Goldenrod,
            "darkgoldenrod" => DarkGoldenrod,
            "peru" => Peru,
            "chocolate" => Chocolate,
            "saddlebrown" => SaddleBrown,
            "sienna" => Sienna,
            "brown" => Brown,
            "maroon" => Maroon,
            "white" => White,
            "snow" => Snow,
            "honeydew" => HoneyDew,
            "mintcream" => MintCream,
            "azure" => Azure,
            "aliceblue" => AliceBlue,
            "ghostwhite" => GhostWhite,
            "whitesmoke" => WhiteSmoke,
            "seashell" => SeaShell,
            "beige" => Beige,
            "oldlace" => OldLace,
            "floralwhite" => FloralWhite,
            "ivory" => Ivory,
            "antiquewhite" => AntiqueWhite,
            "linen" => Linen,
            "lavenderblush" => LavenderBlush,
            "mistyrose" => MistyRose,
            "gainsboro" => Gainsboro,
            "lightgray" => LightGray,
            "silver" => Silver,
            "darkgray" => DarkGray,
            "gray" => Gray,
            "dimgray" => DimGray,
            "lightslategray" => LightSlateGray,
            "slategray" => SlateGray,
            "darkslategray" => DarkSlateGray,
            "black" => Black,
            _ => return None,
        })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Color::*;
        match self {
            HexRGB(r, g, b) => write!(f, "#{:02x}{:02x}{:02x}", r, g, b),
            HexRGBA(r, g, b, a) => write!(f, "#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
            HSL(h, s, l) => write!(f, "hsl({}, {}%, {}%)", h, s, l),
            HSLA(h, s, l, a) => write!(f, "hsla({}, {}%, {}%, {})", h, s, l, a),
            IndianRed => write!(f, "indianred"),
            LightCoral => write!(f, "lightcoral"),
            Salmon => write!(f, "salmon"),
            DarkSalmon => write!(f, "darksalmon"),
            LightSalmon => write!(f, "lightsalmon"),
            Crimson => write!(f, "crimson"),
            Red => write!(f, "red"),
            FireBrick => write!(f, "firebrick"),
            DarkRed => write!(f, "darkred"),
            Pink => write!(f, "pink"),
            LightPink => write!(f, "lightpink"),
            HotPink => write!(f, "hotpink"),
            DeepPink => write!(f, "deeppink"),
            MediumVioletRed => write!(f, "mediumvioletred"),
            PaleVioletRed => write!(f, "palevioletred"),
            Coral => write!(f, "coral"),
            Tomato => write!(f, "tomato"),
            OrangeRed => write!(f, "orangered"),
            DarkOrange => write!(f, "darkorange"),
            Orange => write!(f, "orange"),
            Gold => write!(f, "gold"),
            Yellow => write!(f, "yellow"),
            LightYellow => write!(f, "lightyellow"),
            LemonChiffon => write!(f, "lemonchiffon"),
            LightGoldenrodYellow => write!(f, "lightgoldenrodyellow"),
            PapayaWhip => write!(f, "papayawhip"),
            Moccasin => write!(f, "Moccasin"),
            PeachPuff => write!(f, "Peachpuff"),
            PaleGoldenrod => write!(f, "palegoldenrod"),
            Khaki => write!(f, "khaki"),
            DarkKhaki => write!(f, "darkkhaki"),
            Lavender => write!(f, "lavender"),
            Thistle => write!(f, "thistle"),
            Plum => write!(f, "plum"),
            Violet => write!(f, "violet"),
            Orchid => write!(f, "orchid"),
            Fuchsia => write!(f, "fuchsia"),
            Magenta => write!(f, "magenta"),
            MediumOrchid => write!(f, "mediumorchid"),
            MediumPurple => write!(f, "mediumpurple"),
            RebeccaPurple => write!(f, "rebeccapurple"),
            BlueViolet => write!(f, "blueviolet"),
            DarkViolet => write!(f, "darkviolet"),
            DarkOrchid => write!(f, "darkorchid"),
            DarkMagenta => write!(f, "darkmagenta"),
            Purple => write!(f, "purple"),
            Indigo => write!(f, "indigo"),
            SlateBlue => write!(f, "slateblue"),
            DarkSlateBlue => write!(f, "darkslateblue"),
            MediumSlateBlue => write!(f, "mediumslateblue"),
            GreenYellow => write!(f, "greenyellow"),
            Chartreuse => write!(f, "chartreuse"),
            LawnGreen => write!(f, "lawngreen"),
            Lime => write!(f, "lime"),
            LimeGreen => write!(f, "limegreen"),
            PaleGreen => write!(f, "palegreen"),
            LightGreen => write!(f, "lightgreen"),
            MediumSpringGreen => write!(f, "mediumspringgreen"),
            SpringGreen => write!(f, "springgreen"),
            MediumSeaGreen => write!(f, "mediumseagreen"),
            SeaGreen => write!(f, "seagreen"),
            ForestGreen => write!(f, "forestgreen"),
            Green => write!(f, "green"),
            DarkGreen => write!(f, "darkgreen"),
            YellowGreen => write!(f, "yellowgreen"),
            OliveDrab => write!(f, "olivedrab"),
            Olive => write!(f, "olive"),
            DarkOliveGreen => write!(f, "darkolivegreen"),
            MediumAquamarine => write!(f, "mediumaquamarine"),
            DarkSeaGreen => write!(f, "darkseagreen"),
            LightSeaGreen => write!(f, "lightseagreen"),
            DarkCyan => write!(f, "darkcyan"),
            Teal => write!(f, "teal"),
            Aqua => write!(f, "aqua"),
            Cyan => write!(f, "cyan"),
            LightCyan => write!(f, "lightcyan"),
            PaleTurquoise => write!(f, "paleturquoise"),
            Aquamarine => write!(f, "aquamarine"),
            Turquoise => write!(f, "turquoise"),
            MediumTurquoise => write!(f, "mediumturquoise"),
            DarkTurquoise => write!(f, "darkturquoise"),
            CadetBlue => write!(f, "cadetblue"),
            SteelBlue => write!(f, "steelblue"),
            LightSteelBlue => write!(f, "lightsteelblue"),
            PowderBlue => write!(f, "powderblue"),
            LightBlue => write!(f, "lightblue"),
            SkyBlue => write!(f, "skyblue"),
            LightSkyBlue => write!(f, "lightskyblue"),
            DeepSkyBlue => write!(f, "deepskyblue"),
            DodgerBlue => write!(f, "dodgerblue"),
            CornflowerBlue => write!(f, "cornflowerblue"),
            RoyalBlue => write!(f, "royalblue"),
            Blue => write!(f, "blue"),
            MediumBlue => write!(f, "mediumblue"),
            DarkBlue => write!(f, "darkblue"),
            Navy => write!(f, "navy"),
            MidnightBlue => write!(f, "midnightblue"),
            Cornsilk => write!(f, "cornsilk"),
            BlanchedAlmond => write!(f, "blanchedalmond"),
            Bisque => write!(f, "bisque"),
            NavajoWhite => write!(f, "navajowhite"),
            Wheat => write!(f, "wheat"),
            BurlyWood => write!(f, "burlywood"),
            Tan => write!(f, "tan"),
            RosyBrown => write!(f, "rosybrown"),
            SandyBrown => write!(f, "sandybrown"),
            Goldenrod => write!(f, "goldenrod"),
            DarkGoldenrod => write!(f, "darkgoldenrod"),
            Peru => write!(f, "peru"),
            Chocolate => write!(f, "chocolate"),
            SaddleBrown => write!(f, "saddlebrown"),
            Sienna => write!(f, "sienna"),
            Brown => write!(f, "brown"),
            Maroon => write!(f, "maroon"),
            White => write!(f, "white"),
            Snow => write!(f, "snow"),
            HoneyDew => write!(f, "honeydew"),
            MintCream => write!(f, "mintcream"),
            Azure => write!(f, "azure"),
            AliceBlue => write!(f, "aliceblue"),
            GhostWhite => write!(f, "ghostwhite"),
            WhiteSmoke => write!(f, "whitesmoke"),
            SeaShell => write!(f, "seashell"),
            Beige => write!(f, "beige"),
            OldLace => write!(f, "oldlace"),
            FloralWhite => write!(f, "floralwhite"),
            Ivory => write!(f, "ivory"),
            AntiqueWhite => write!(f, "antiquewhite"),
            Linen => write!(f, "linen"),
            LavenderBlush => write!(f, "lavenderblush"),
            MistyRose => write!(f, "mistyrose"),
            Gainsboro => write!(f, "gainsboro"),
            LightGray => write!(f, "lightgray"),
            Silver => write!(f, "silver"),
            DarkGray => write!(f, "darkgray"),
            Gray => write!(f, "gray"),
            DimGray => write!(f, "dimgray"),
            LightSlateGray => write!(f, "lightslategray"),
            SlateGray => write!(f, "slategray"),
            DarkSlateGray => write!(f, "darkslategray"),
            Black => write!(f, "black"),
        }
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    debug_assert!(h >= 0.0 && h < 360.0);
    debug_assert!(s >= 0.0 && s <= 1.0);
    debug_assert!(l >= 0.0 && l <= 1.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c * 0.5;
    let (rp, gp, bp) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (rp + m, gp + m, bp + m)
}

pub fn parse_hex(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(hex.get(0..1)?, 16).ok()?;
            let g = u8::from_str_radix(hex.get(1..2)?, 16).ok()?;
            let b = u8::from_str_radix(hex.get(2..3)?, 16).ok()?;
            // #fff is equivalent to #ffffff
            Some(Color::HexRGB(r << 4 | r, g << 4 | g, b << 4 | b))
        }
        6 => {
            let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
            let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
            let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
            Some(Color::HexRGB(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
            let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
            let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
            let a = u8::from_str_radix(hex.get(6..8)?, 16).ok()?;
            Some(Color::HexRGBA(r, g, b, a))
        }
        _ => None,
    }
}

#[test]
fn test_color_convert() {
    let color = Color::HSL(60.0, 0.0, 100.0);
    assert_eq!(color.to_rgb(), Color::HexRGB(255, 255, 255));
    let color = Color::HSL(0.0, 100.0, 50.0);
    assert_eq!(color.to_rgb(), Color::HexRGB(255, 0, 0));
}
