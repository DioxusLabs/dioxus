use std::{num::ParseFloatError, str::FromStr};

use ratatui::style::{Color, Modifier, Style};

use crate::RenderingMode;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RinkColor {
    pub color: Color,
    pub alpha: u8,
}

impl Default for RinkColor {
    fn default() -> Self {
        Self {
            color: Color::Black,
            alpha: 0,
        }
    }
}

impl RinkColor {
    pub fn blend(self, other: Color) -> Color {
        if self.color == Color::Reset {
            Color::Reset
        } else if self.alpha == 0 {
            other
        } else {
            let [sr, sg, sb] = to_rgb(self.color).map(|e| e as u16);
            let [or, og, ob] = to_rgb(other).map(|e| e as u16);
            let sa = self.alpha as u16;
            let rsa = 255 - sa;
            Color::Rgb(
                ((sr * sa + or * rsa) / 255) as u8,
                ((sg * sa + og * rsa) / 255) as u8,
                ((sb * sa + ob * rsa) / 255) as u8,
            )
        }
    }
}

fn parse_value(
    v: &str,
    current_max_output: f32,
    required_max_output: f32,
) -> Result<f32, ParseFloatError> {
    if let Some(stripped) = v.strip_suffix('%') {
        Ok((stripped.trim().parse::<f32>()? / 100.0) * required_max_output)
    } else {
        Ok((v.trim().parse::<f32>()? / current_max_output) * required_max_output)
    }
}

pub struct ParseColorError;

fn parse_hex(color: &str) -> Result<Color, ParseColorError> {
    let mut values = [0, 0, 0];
    let mut color_ok = true;
    for i in 0..values.len() {
        if let Ok(v) = u8::from_str_radix(&color[(1 + 2 * i)..(1 + 2 * (i + 1))], 16) {
            values[i] = v;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        Ok(Color::Rgb(values[0], values[1], values[2]))
    } else {
        Err(ParseColorError)
    }
}

fn parse_rgb(color: &str) -> Result<Color, ParseColorError> {
    let mut values = [0, 0, 0];
    let mut color_ok = true;
    for (v, i) in color.split(',').zip(0..values.len()) {
        if let Ok(v) = parse_value(v.trim(), 255.0, 255.0) {
            values[i] = v as u8;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        Ok(Color::Rgb(values[0], values[1], values[2]))
    } else {
        Err(ParseColorError)
    }
}

fn parse_hsl(color: &str) -> Result<Color, ParseColorError> {
    let mut values = [0.0, 0.0, 0.0];
    let mut color_ok = true;
    for (v, i) in color.split(',').zip(0..values.len()) {
        if let Ok(v) = parse_value(v.trim(), if i == 0 { 360.0 } else { 100.0 }, 1.0) {
            values[i] = v;
        } else {
            color_ok = false;
        }
    }
    if color_ok {
        let [h, s, l] = values;
        let rgb = if s == 0.0 {
            [l as u8; 3]
        } else {
            fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
                if t < 0.0 {
                    t += 1.0;
                }
                if t > 1.0 {
                    t -= 1.0;
                }
                if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 1.0 / 2.0 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                }
            }

            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            [
                (hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0) as u8,
                (hue_to_rgb(p, q, h) * 255.0) as u8,
                (hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0) as u8,
            ]
        };

        Ok(Color::Rgb(rgb[0], rgb[1], rgb[2]))
    } else {
        Err(ParseColorError)
    }
}

impl FromStr for RinkColor {
    type Err = ParseColorError;

    fn from_str(color: &str) -> Result<Self, Self::Err> {
        match color {
            "red" => Ok(RinkColor {
                color: Color::Red,
                alpha: 255,
            }),
            "black" => Ok(RinkColor {
                color: Color::Black,
                alpha: 255,
            }),
            "green" => Ok(RinkColor {
                color: Color::Green,
                alpha: 255,
            }),
            "yellow" => Ok(RinkColor {
                color: Color::Yellow,
                alpha: 255,
            }),
            "blue" => Ok(RinkColor {
                color: Color::Blue,
                alpha: 255,
            }),
            "magenta" => Ok(RinkColor {
                color: Color::Magenta,
                alpha: 255,
            }),
            "cyan" => Ok(RinkColor {
                color: Color::Cyan,
                alpha: 255,
            }),
            "gray" => Ok(RinkColor {
                color: Color::Gray,
                alpha: 255,
            }),
            "darkgray" => Ok(RinkColor {
                color: Color::DarkGray,
                alpha: 255,
            }),
            // light red does not exist
            "orangered" => Ok(RinkColor {
                color: Color::LightRed,
                alpha: 255,
            }),
            "lightgreen" => Ok(RinkColor {
                color: Color::LightGreen,
                alpha: 255,
            }),
            "lightyellow" => Ok(RinkColor {
                color: Color::LightYellow,
                alpha: 255,
            }),
            "lightblue" => Ok(RinkColor {
                color: Color::LightBlue,
                alpha: 255,
            }),
            // light magenta does not exist
            "orchid" => Ok(RinkColor {
                color: Color::LightMagenta,
                alpha: 255,
            }),
            "lightcyan" => Ok(RinkColor {
                color: Color::LightCyan,
                alpha: 255,
            }),
            "white" => Ok(RinkColor {
                color: Color::White,
                alpha: 255,
            }),
            _ => {
                if color.len() == 7 && color.starts_with('#') {
                    parse_hex(color).map(|c| RinkColor {
                        color: c,
                        alpha: 255,
                    })
                } else if let Some(stripped) = color.strip_prefix("rgb(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (alpha, rgb_values) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = alpha.parse() {
                            parse_rgb(rgb_values).map(|c| RinkColor { color: c, alpha: a })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_rgb(color_values).map(|c| RinkColor {
                            color: c,
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("rgba(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_rgb(rgb_values).map(|c| RinkColor {
                                color: c,
                                alpha: (a * 255.0) as u8,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_rgb(color_values).map(|c| RinkColor {
                            color: c,
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("hsl(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_hsl(rgb_values).map(|c| RinkColor {
                                color: c,
                                alpha: (a * 255.0) as u8,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_hsl(color_values).map(|c| RinkColor {
                            color: c,
                            alpha: 255,
                        })
                    }
                } else if let Some(stripped) = color.strip_prefix("hsla(") {
                    let color_values = stripped.trim_end_matches(')');
                    if color.matches(',').count() == 3 {
                        let (rgb_values, alpha) =
                            color_values.rsplit_once(',').ok_or(ParseColorError)?;
                        if let Ok(a) = parse_value(alpha, 1.0, 1.0) {
                            parse_hsl(rgb_values).map(|c| RinkColor {
                                color: c,
                                alpha: (a * 255.0) as u8,
                            })
                        } else {
                            Err(ParseColorError)
                        }
                    } else {
                        parse_hsl(color_values).map(|c| RinkColor {
                            color: c,
                            alpha: 255,
                        })
                    }
                } else {
                    Err(ParseColorError)
                }
            }
        }
    }
}

fn to_rgb(c: Color) -> [u8; 3] {
    match c {
        Color::Black => [0, 0, 0],
        Color::Red => [255, 0, 0],
        Color::Green => [0, 128, 0],
        Color::Yellow => [255, 255, 0],
        Color::Blue => [0, 0, 255],
        Color::Magenta => [255, 0, 255],
        Color::Cyan => [0, 255, 255],
        Color::Gray => [128, 128, 128],
        Color::DarkGray => [169, 169, 169],
        Color::LightRed => [255, 69, 0],
        Color::LightGreen => [144, 238, 144],
        Color::LightYellow => [255, 255, 224],
        Color::LightBlue => [173, 216, 230],
        Color::LightMagenta => [218, 112, 214],
        Color::LightCyan => [224, 255, 255],
        Color::White => [255, 255, 255],
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(idx) => match idx {
            16..=231 => {
                let v = idx - 16;
                // add 3 to round up
                let r = ((v as u16 / 36) * 255 + 3) / 5;
                let g = (((v as u16 % 36) / 6) * 255 + 3) / 5;
                let b = ((v as u16 % 6) * 255 + 3) / 5;
                [r as u8, g as u8, b as u8]
            }
            232..=255 => {
                let l = (idx - 232) / 24;
                [l; 3]
            }
            // plasmo will never generate these colors, but they might be on the screen from another program
            _ => [0, 0, 0],
        },
        Color::Reset => [0, 0, 0],
    }
}

pub fn convert(mode: RenderingMode, c: Color) -> Color {
    if let Color::Reset = c {
        c
    } else {
        match mode {
            crate::RenderingMode::BaseColors => match c {
                Color::Rgb(_, _, _) => panic!("cannot convert rgb color to base color"),
                Color::Indexed(_) => panic!("cannot convert Ansi color to base color"),
                _ => c,
            },
            crate::RenderingMode::Rgb => {
                let rgb = to_rgb(c);
                Color::Rgb(rgb[0], rgb[1], rgb[2])
            }
            crate::RenderingMode::Ansi => match c {
                Color::Indexed(_) => c,
                _ => {
                    let rgb = to_rgb(c);
                    // 16-231: 6 × 6 × 6 color cube
                    // 232-255: 23 step grayscale
                    if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
                        let idx = 232 + (rgb[0] as u16 * 23 / 255) as u8;
                        Color::Indexed(idx)
                    } else {
                        let r = (rgb[0] as u16 * 5) / 255;
                        let g = (rgb[1] as u16 * 5) / 255;
                        let b = (rgb[2] as u16 * 5) / 255;
                        let idx = 16 + r * 36 + g * 6 + b;
                        Color::Indexed(idx as u8)
                    }
                }
            },
        }
    }
}

#[test]
fn rgb_to_ansi() {
    for idx in 17..=231 {
        let idxed = Color::Indexed(idx);
        let rgb = to_rgb(idxed);
        // gray scale colors have two equivelent repersentations
        let color = Color::Rgb(rgb[0], rgb[1], rgb[2]);
        let converted = convert(RenderingMode::Ansi, color);
        if let Color::Indexed(i) = converted {
            if rgb[0] != rgb[1] || rgb[1] != rgb[2] {
                assert_eq!(idxed, converted);
            } else {
                assert!(i >= 232);
            }
        } else {
            panic!("color is not indexed")
        }
    }
    for idx in 232..=255 {
        let idxed = Color::Indexed(idx);
        let rgb = to_rgb(idxed);
        assert!(rgb[0] == rgb[1] && rgb[1] == rgb[2]);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RinkStyle {
    pub fg: Option<RinkColor>,
    pub bg: Option<RinkColor>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}

impl Default for RinkStyle {
    fn default() -> Self {
        Self {
            fg: Some(RinkColor {
                color: Color::White,
                alpha: 255,
            }),
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        }
    }
}

impl RinkStyle {
    pub fn add_modifier(mut self, m: Modifier) -> Self {
        self.sub_modifier.remove(m);
        self.add_modifier.insert(m);
        self
    }

    pub fn remove_modifier(mut self, m: Modifier) -> Self {
        self.add_modifier.remove(m);
        self.sub_modifier.insert(m);
        self
    }

    pub fn merge(mut self, other: RinkStyle) -> Self {
        self.fg = self.fg.or(other.fg);
        self.add_modifier(other.add_modifier)
            .remove_modifier(other.sub_modifier)
    }
}

impl From<RinkStyle> for Style {
    fn from(val: RinkStyle) -> Self {
        Style {
            underline_color: None,
            fg: val.fg.map(|c| c.color),
            bg: val.bg.map(|c| c.color),
            add_modifier: val.add_modifier,
            sub_modifier: val.sub_modifier,
        }
    }
}
