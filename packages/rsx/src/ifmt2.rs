use std::str::FromStr;

use proc_macro2::TokenStream as TokenStream2;
use syn::{parse::Parse, token::Token, LitStr};

struct Ifmt2 {
    raw: LitStr,
}

struct IfmtParser {}
impl syn::parse::Parser for IfmtParser {
    type Output = ();

    fn parse2(self, tokens: proc_macro2::TokenStream) -> syn::Result<Self::Output> {
        dbg!(&tokens);

        for token in tokens {
            dbg!(token.to_string().len(), token);
        }

        Ok(())
    }
}

impl Parse for Ifmt2 {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let raw: LitStr = input.parse()?;

        let split = raw.value();

        for item in split.split_ascii_whitespace() {
            dbg!(item);
            let parsed = TokenStream2::from_str(item);
            match parsed {
                Ok(parsed) => {
                    dbg!(parsed);
                }
                Err(e) => {
                    dbg!(e);
                }
            }
        }

        // let ifmtparser = IfmtParser {};

        // let out = raw.parse_with(ifmtparser).unwrap();

        Ok(Self { raw })
    }
}

#[test]
fn parses() {
    let toks = quote::quote! {
        "something {cool} {{cooler}} not cool \n\r\t a\u{0302} 😀 🚀 ✨ 🐍 ❤️ #️⃣"
    };

    // let a = "This string uses CRLF line endings.\r\nSecond line.\r\n";
    // let a = "This string uses LF line endings.\nSecond line.\n";
    // let a = "This string uses CR line endings.\rSecond line.\r";
    // let a = "e\u{0301} (e with acute), a\u{0302} (a with circumflex)";
    // let a = "Emojis and symbols: 😀 🚀 ✨ 🐍 ❤️ #️⃣";

    let parsed = syn::parse2::<Ifmt2>(toks).unwrap();
}

fn parse_string_literal(input: &str) -> String {
    if input.starts_with('r') {
        // Parse raw string literal
        let start_hashes = input.find('"').unwrap();
        let end_hashes = input.rfind('"').unwrap();
        let raw_content = &input[start_hashes + 1..end_hashes];
        return raw_content.to_string();
    }

    let mut output = String::new();
    let mut chars = input.chars().peekable();

    // Skip the starting and ending quotes
    chars.next();
    chars.next_back();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_char) = chars.next() {
                match next_char {
                    'n' => output.push('\n'),
                    't' => output.push('\t'),
                    'r' => output.push('\r'),
                    '\\' => output.push('\\'),
                    '"' => output.push('"'),
                    '0' => output.push('\0'),
                    'x' => {
                        // Parse hex escape
                        let hex1 = chars.next().unwrap();
                        let hex2 = chars.next().unwrap();
                        let hex = format!("{}{}", hex1, hex2);
                        let byte = u8::from_str_radix(&hex, 16).unwrap();
                        output.push(byte as char);
                    }
                    'u' => {
                        // Parse Unicode escape
                        assert_eq!(chars.next(), Some('{'));
                        let mut unicode = String::new();
                        while let Some(c) = chars.next() {
                            if c == '}' {
                                break;
                            }
                            unicode.push(c);
                        }
                        let code_point = u32::from_str_radix(&unicode, 16).unwrap();
                        let ch = char::from_u32(code_point).unwrap();
                        output.push(ch);
                    }
                    _ => panic!("Unknown escape sequence: \\{}", next_char),
                }
            }
        } else {
            output.push(c);
        }
    }

    output
}

macro_rules! does_nothing {
    ($($item:tt)*) => {
        {
            let _ = format!($($item)*);
            ($($item)*)
        }
    };
}

macro_rules! fmt_all {
    ($($fmt:expr),* $(,)?) => {
        vec![
            $(
                does_nothing!($fmt)
            ),*
        ]
    };
}

// #[testa]
fn main2() {
    // Example use of the macro
    let a1234 = 10;
    let b123 = 20;

    let args = fmt_all!("something {a1234}", "something {b123}",);

    // Print each format argument
    for arg in args {
        // println!("{}", arg);
    }
}
// #[test]
fn literal_parser() {
    let test_cases = [
        r#""This is a newline character: \n This is a tab character: \t""#,
        r#""Unicode test: \u{1F60A} \u{1F4A9} \u{2603}""#,
        r#""Quotes: \"double quotes\", 'single quotes', backslash: \\""#,
        r##"r#"This is a raw string literal. No need to escape "quotes" or \backslashes\"#"##,
        r###"r#"Raw string with "quotes" and #hashtags## inside"#"###,
        r#""This is a multiline string.
        // It spans multiple lines.
        // Here is another line.""#,
        r#""Special characters: \x41 \x42 \x43 \t \n \r""#,
        r#"This is a string with special characters \u{1F60A}, escape sequences \n\t, and raw segments: r#\"raw string\"#,
        r#""""#,
        r#""This string contains a null character.\0 End of string.""#,
        r#""This string uses CRLF line endings.\r\nSecond line.\r\n""#,
        r#""This string uses LF line endings.\nSecond line.\n""#,
        r#""This string uses CR line endings.\rSecond line.\r""#,
        r#""e\u{0301} (e with acute), a\u{0302} (a with circumflex)""#,
        r#""Emojis and symbols: 😀 🚀 ✨ 🐍 ❤️ #️⃣""#,
        r#""\x01\x02\x03\x04\x05\x06\x07\x08\x0B\x0C\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F""#,
    ];

    let cool3 = 123;
    let fmted = format_args!("Something {cool3}");
    // let does = passthru_tofmtargs!("Something {cool3}");

    // for &test_case in &test_cases {
    //     println!(
    //         "Input: {}\nParsed: {}\n",
    //         test_case,
    //         parse_string_literal(test_case)
    //     );
    // }
}
