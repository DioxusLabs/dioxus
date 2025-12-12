// pub use class_name_pattern::{ClassNamePattern, Fragment};

use std::{
    borrow::Cow,
    fs,
    hash::{Hash as _, Hasher as _},
    path::{Path, PathBuf},
    str::FromStr,
};

use siphasher::sip::SipHasher13;
use winnow::{
    combinator::{alt, cut_err, delimited, fold_repeat, opt, peek, preceded, terminated},
    error::{ContextError, ParseError},
    stream::{AsChar, ContainsToken, Range},
    token::{none_of, one_of, tag, take_till, take_until0, take_while},
    PResult, Parser,
};

/// ```text
///         v----v inner span
/// :global(.class)
/// ^-------------^ outer span
/// ```
#[derive(Debug, PartialEq)]
pub struct Global<'s> {
    pub inner: &'s str,
    pub outer: &'s str,
}

#[derive(Debug, PartialEq)]
pub enum CssFragment<'s> {
    Class(&'s str),
    Global(Global<'s>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Fragment {
    Str(String),
    Name,
    Hash,
}

/// Pattern to create new class names
#[derive(Debug, Clone, PartialEq)]
pub struct ClassNamePattern(Vec<Fragment>);

impl ClassNamePattern {
    pub fn apply(&self, classname: &str, hash: &str) -> String {
        self.0
            .iter()
            .map(|v| match v {
                Fragment::Str(s) => s,
                Fragment::Name => classname,
                Fragment::Hash => hash,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

impl Default for ClassNamePattern {
    fn default() -> Self {
        Self(vec![
            Fragment::Name,
            Fragment::Str("-".into()),
            Fragment::Hash,
        ])
    }
}

//************************************************************************//

pub fn create_hash(css: &str, hash_len: usize) -> String {
    let mut hasher = SipHasher13::new();
    css.hash(&mut hasher);
    let hash = hasher.finish();
    let mut hash_str = format!("{hash:x}");
    hash_str.truncate(hash_len);
    hash_str
}

/// Parses and rewrites CSS class selectors
pub fn transform_css<'a>(
    css: &'a str,
    class_name_pattern: &ClassNamePattern,
    hash: &str,
) -> Result<String, ParseError<&'a str, ContextError>> {
    let fragments = parse_css(&css)?;

    let mut new_css = String::with_capacity(css.len() * 2);
    let mut cursor = css;

    for fragment in fragments {
        let (span, replace) = match fragment {
            CssFragment::Class(class) => (class, Cow::Owned(class_name_pattern.apply(class, hash))),
            CssFragment::Global(Global { inner, outer }) => (outer, Cow::Borrowed(inner)),
        };

        let (before, after) = cursor.split_at(span.as_ptr() as usize - cursor.as_ptr() as usize);
        cursor = &after[span.len()..];
        new_css.push_str(before);
        new_css.push_str(&replace);
    }

    new_css.push_str(cursor);
    Ok(new_css)
}

pub fn get_class_mappings<'a>(
    css: &'a str,
    class_name_pattern: &ClassNamePattern,
    hash: &str,
) -> Result<Vec<(&'a str, Cow<'a, str>)>, ParseError<&'a str, ContextError>> {
    let fragments = parse_css(&css)?;
    let mut result = Vec::new();

    for c in fragments {
        match c {
            CssFragment::Class(class) => {
                result.push((class, Cow::Owned(class_name_pattern.apply(class, hash))));
            }
            CssFragment::Global(global) => {
                let global_classes = resolve_global_inner_classes(global)?;
                result.extend(
                    global_classes
                        .into_iter()
                        .map(|class| (class, Cow::Borrowed(class))),
                );
            }
        }
    }
    result.sort_by_key(|e| e.0);
    result.dedup_by_key(|e| e.0);
    Ok(result)
}

fn resolve_global_inner_classes<'a>(
    global: Global<'a>,
) -> Result<Vec<&'a str>, ParseError<&'a str, ContextError>> {
    let mut input = global.inner;
    let fragments = selector.parse(&mut input)?;
    let mut result = Vec::new();
    for c in fragments {
        match c {
            CssFragment::Class(class) => result.push(class),
            CssFragment::Global(_) => {
                unreachable!("Top level parser should have already errored if globals are nested")
            }
        }
    }
    Ok(result)
}

//************************************************************************//

pub fn parse_css(input: &str) -> Result<Vec<CssFragment<'_>>, ParseError<&str, ContextError>> {
    style_rule_block_contents.parse(input)
}

pub fn recognize_repeat<'s, O>(
    range: impl Into<Range>,
    f: impl Parser<&'s str, O, ContextError>,
) -> impl Parser<&'s str, &'s str, ContextError> {
    fold_repeat(range, f, || (), |_, _| ()).recognize()
}

fn ws<'s>(input: &mut &'s str) -> PResult<&'s str> {
    recognize_repeat(
        0..,
        alt((
            line_comment,
            block_comment,
            take_while(1.., (AsChar::is_space, '\n', '\r')),
        )),
    )
    .parse_next(input)
}

fn line_comment<'s>(input: &mut &'s str) -> PResult<&'s str> {
    ("//", take_while(0.., |c| c != '\n'))
        .recognize()
        .parse_next(input)
}

fn block_comment<'s>(input: &mut &'s str) -> PResult<&'s str> {
    ("/*", cut_err(terminated(take_until0("*/"), "*/")))
        .recognize()
        .parse_next(input)
}

// matches a sass interpolation of the form #{...}
fn sass_interpolation<'s>(input: &mut &'s str) -> PResult<&'s str> {
    (
        "#{",
        cut_err(terminated(take_till(1.., ('{', '}', '\n')), '}')),
    )
        .recognize()
        .parse_next(input)
}

fn identifier<'s>(input: &mut &'s str) -> PResult<&'s str> {
    (
        one_of(('_', '-', AsChar::is_alpha)),
        take_while(0.., ('_', '-', AsChar::is_alphanum)),
    )
        .recognize()
        .parse_next(input)
}

fn class<'s>(input: &mut &'s str) -> PResult<&'s str> {
    preceded('.', identifier).parse_next(input)
}

fn global<'s>(input: &mut &'s str) -> PResult<Global<'s>> {
    let (inner, outer) = preceded(
        ":global(",
        cut_err(terminated(
            stuff_till(0.., (')', '(', '{')), // inner
            ')',
        )),
    )
    .with_recognized() // outer
    .parse_next(input)?;
    Ok(Global { inner, outer })
}

fn string_dq<'s>(input: &mut &'s str) -> PResult<&'s str> {
    let str_char = alt((none_of(['"']).void(), tag("\\\"").void()));
    let str_chars = recognize_repeat(0.., str_char);

    preceded('"', cut_err(terminated(str_chars, '"'))).parse_next(input)
}

fn string_sq<'s>(input: &mut &'s str) -> PResult<&'s str> {
    let str_char = alt((none_of(['\'']).void(), tag("\\'").void()));
    let str_chars = recognize_repeat(0.., str_char);

    preceded('\'', cut_err(terminated(str_chars, '\''))).parse_next(input)
}

fn string<'s>(input: &mut &'s str) -> PResult<&'s str> {
    alt((string_dq, string_sq)).parse_next(input)
}

/// Behaves like take_till except it finds and parses strings and
/// comments (allowing those to contain the end condition characters).
pub fn stuff_till<'s>(
    range: impl Into<Range>,
    list: impl ContainsToken<char>,
) -> impl Parser<&'s str, &'s str, ContextError> {
    recognize_repeat(
        range,
        alt((
            string.void(),
            block_comment.void(),
            line_comment.void(),
            sass_interpolation.void(),
            '/'.void(),
            '#'.void(),
            take_till(1.., ('\'', '"', '/', '#', list)).void(),
        )),
    )
}

pub(crate) fn selector<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    fold_repeat(
        1..,
        alt((
            class.map(|c| Some(CssFragment::Class(c))),
            global.map(|g| Some(CssFragment::Global(g))),
            ':'.map(|_| None),
            stuff_till(1.., ('.', ';', '{', '}', ':')).map(|_| None),
        )),
        Vec::new,
        |mut acc, item| {
            if let Some(item) = item {
                acc.push(item);
            }
            acc
        },
    )
    .parse_next(input)
}

fn declaration<'s>(input: &mut &'s str) -> PResult<&'s str> {
    (
        (opt('$'), identifier),
        ws,
        ':',
        terminated(
            stuff_till(1.., (';', '{', '}')),
            alt((';', peek('}'))), // semicolon is optional if it's the last element in a rule block
        ),
    )
        .recognize()
        .parse_next(input)
}

fn style_rule_block_statement<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    let content = alt((
        declaration.map(|_| Vec::new()), //
        at_rule,
        style_rule,
    ));
    delimited(ws, content, ws).parse_next(input)
}

fn style_rule_block_contents<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    fold_repeat(
        0..,
        style_rule_block_statement,
        Vec::new,
        |mut acc, mut item| {
            acc.append(&mut item);
            acc
        },
    )
    .parse_next(input)
}

fn style_rule_block<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    preceded(
        '{',
        cut_err(terminated(style_rule_block_contents, (ws, '}'))),
    )
    .parse_next(input)
}

fn style_rule<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    let (mut classes, mut nested_classes) = (selector, style_rule_block).parse_next(input)?;
    classes.append(&mut nested_classes);
    Ok(classes)
}

fn at_rule<'s>(input: &mut &'s str) -> PResult<Vec<CssFragment<'s>>> {
    let (identifier, char) = preceded(
        '@',
        cut_err((
            terminated(identifier, stuff_till(0.., ('{', '}', ';'))),
            alt(('{', ';', peek('}'))),
        )),
    )
    .parse_next(input)?;

    if char != '{' {
        return Ok(vec![]);
    }

    match identifier {
        "media" | "layer" | "container" | "include" => {
            cut_err(terminated(style_rule_block_contents, '}')).parse_next(input)
        }
        _ => {
            cut_err(terminated(unknown_block_contents, '}')).parse_next(input)?;
            Ok(vec![])
        }
    }
    // if identifier == "media" {
    //     cut_err(terminated(style_rule_block_contents, '}')).parse_next(input)
    // } else {
    //     cut_err(terminated(unknown_block_contents, '}')).parse_next(input)?;
    //     Ok(vec![])
    // }
}

fn unknown_block_contents<'s>(input: &mut &'s str) -> PResult<&'s str> {
    recognize_repeat(
        0..,
        alt((
            stuff_till(1.., ('{', '}')).void(),
            ('{', cut_err((unknown_block_contents, '}'))).void(),
        )),
    )
    .parse_next(input)
}

//************************************************************************//

#[test]
fn test_class() {
    let mut input = "._x1a2b Hello";

    let r = class.parse_next(&mut input);
    assert_eq!(r, Ok("_x1a2b"));
}

#[test]
fn test_selector() {
    let mut input = ".foo.bar [value=\"fa.sdasd\"] /* .banana */ // .apple \n \t .cry {";

    let r = selector.parse_next(&mut input);
    assert_eq!(
        r,
        Ok(vec![
            CssFragment::Class("foo"),
            CssFragment::Class("bar"),
            CssFragment::Class("cry")
        ])
    );

    let mut input = "{";

    let r = selector.recognize().parse_next(&mut input);
    assert!(r.is_err());
}

#[test]
fn test_declaration() {
    let mut input = "background-color \t : red;";

    let r = declaration.parse_next(&mut input);
    assert_eq!(r, Ok("background-color \t : red;"));

    let r = declaration.parse_next(&mut input);
    assert!(r.is_err());
}

#[test]
fn test_style_rule() {
    let mut input = ".foo.bar {
        background-color: red;
        .baz {
            color: blue;
        }
        $some-scss-var: 10px;
        @some-at-rule blah blah;
        @media blah .blah {
            .moo {
                color: red;
            }
        }
        @container (width > 700px) {
            .zoo {
                color: blue;
            }
        }
    }END";

    let r = style_rule.parse_next(&mut input);
    assert_eq!(
        r,
        Ok(vec![
            CssFragment::Class("foo"),
            CssFragment::Class("bar"),
            CssFragment::Class("baz"),
            CssFragment::Class("moo"),
            CssFragment::Class("zoo")
        ])
    );

    assert_eq!(input, "END");
}

#[test]
fn test_at_rule_simple() {
    let mut input = "@simple-rule blah \"asd;asd\" blah;";

    let r = at_rule.parse_next(&mut input);
    assert_eq!(r, Ok(vec![]));

    assert!(input.is_empty());
}

#[test]
fn test_at_rule_unknown() {
    let mut input = "@unknown blah \"asdasd\" blah {
        bunch of stuff {
            // things inside {
            blah
            ' { '
        }

        .bar {
            color: blue;

            .baz {
                color: green;
            }
        }
    }";

    let r = at_rule.parse_next(&mut input);
    assert_eq!(r, Ok(vec![]));

    assert!(input.is_empty());
}

#[test]
fn test_at_rule_media() {
    let mut input = "@media blah \"asdasd\" blah {
        .foo {
            background-color: red;
        }

        .bar {
            color: blue;

            .baz {
                color: green;
            }
        }
    }";

    let r = at_rule.parse_next(&mut input);
    assert_eq!(
        r,
        Ok(vec![
            CssFragment::Class("foo"),
            CssFragment::Class("bar"),
            CssFragment::Class("baz")
        ])
    );

    assert!(input.is_empty());
}

#[test]
fn test_at_rule_layer() {
    let mut input = "@layer test {
        .foo {
            background-color: red;
        }

        .bar {
            color: blue;

            .baz {
                color: green;
            }
        }
    }";

    let r = at_rule.parse_next(&mut input);
    assert_eq!(
        r,
        Ok(vec![
            CssFragment::Class("foo"),
            CssFragment::Class("bar"),
            CssFragment::Class("baz")
        ])
    );

    assert!(input.is_empty());
}

#[test]
fn test_top_level() {
    let mut input = "// tool.module.scss

        .default_border {
          border-color: lch(100% 10 10);
          border-style: dashed double;
          border-radius: 30px;
        
        }
        
        @media testing {
            .media-foo {
                color: red;
            }
        }

        @layer {
            .layer-foo {
                color: blue;
            }
        }

        @include mixin {
            border: none;

            .include-foo {
                color: green;
            }
        }

        @layer foo;

        @debug 1+2 * 3==1+(2 * 3); // true

        .container {
          padding: 1em;
          border: 2px solid;
          border-color: lch(100% 10 10);
          border-style: dashed double;
          border-radius: 30px;
          margin: 1em;
          background-color: lch(45% 9.5 140.4);

          .bar {
            color: red;
          }
        }
        
        @debug 1+2 * 3==1+(2 * 3); // true
        ";

    let r = style_rule_block_contents.parse_next(&mut input);
    assert_eq!(
        r,
        Ok(vec![
            CssFragment::Class("default_border"),
            CssFragment::Class("media-foo"),
            CssFragment::Class("layer-foo"),
            CssFragment::Class("include-foo"),
            CssFragment::Class("container"),
            CssFragment::Class("bar"),
        ])
    );

    println!("{input}");
    assert!(input.is_empty());
}

#[test]
fn test_sass_interpolation() {
    let mut input = "#{$test-test}END";

    let r = sass_interpolation.parse_next(&mut input);
    assert_eq!(r, Ok("#{$test-test}"));

    assert_eq!(input, "END");

    let mut input = "#{$test-test
        }END";
    let r = sass_interpolation.parse_next(&mut input);
    assert!(r.is_err());

    let mut input = "#{$test-test";
    let r = sass_interpolation.parse_next(&mut input);
    assert!(r.is_err());

    let mut input = "#{$test-te{st}";
    let r = sass_interpolation.parse_next(&mut input);
    assert!(r.is_err());
}

#[test]
fn test_get_class_mappings() {
    let css = r#".foo.bar {
        background-color: red;
        :global(.baz) {
            color: blue;
        }
        :global(.bag .biz) {
            color: blue;
        }
        .zig {

        }
        .bong {}
        .zig {
            color: blue;
        }
    }"#;
    let pattern = ClassNamePattern::default();
    let hash = "abc1234";
    let mappings = get_class_mappings(css, &pattern, hash).unwrap();
    let expected = vec![
        ("bag", "bag"),
        ("bar", "bar-abc1234"),
        ("baz", "baz"),
        ("biz", "biz"),
        ("bong", "bong-abc1234"),
        ("foo", "foo-abc1234"),
        ("zig", "zig-abc1234"),
    ];
    if mappings.len() != expected.len() {
        panic!(
            "Expected {} mappings, got {}",
            expected.len(),
            mappings.len()
        );
    }
    for (i, (original, hashed)) in mappings.iter().enumerate() {
        assert_eq!(expected[i].0, *original);
        assert_eq!(expected[i].1, *hashed);
    }
}

#[test]
fn test_parser_error_on_nested_globals() {
    let css = r#".foo :global(.bar .baz) {
        color: blue;
    }"#;
    let result = parse_css(css);
    assert!(result.is_ok());
    let css = r#".foo :global(.bar :global(.baz)) {
        color: blue;
    }"#;
    let result = parse_css(css);
    assert!(result.is_err());
}

#[test]
#[should_panic]
fn test_resolve_global_inner_classes_nested() {
    let global = Global {
        inner: ".foo :global(.bar)".into(),
        outer: ":global(.foo :global(.bar))".into(),
    };
    let _ = resolve_global_inner_classes(global);
}
