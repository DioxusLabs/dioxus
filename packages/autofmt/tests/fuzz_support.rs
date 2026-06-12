//! Shared harness for fuzzing the rsx autoformatter.
//!
//! Generates random-but-valid rust files containing rsx! macros and verifies a set of
//! invariants that the formatter must uphold:
//!
//! 1. Formatting never panics or errors on valid input
//! 2. The formatted output still parses as a valid rust file
//! 3. Formatting is idempotent: fmt(fmt(x)) == fmt(x)
//! 4. No tokens are lost or corrupted by formatting
//! 5. Comments inside rsx! blocks are preserved
//!
//! The generator covers the shapes from historical autofmt issues, including
//! multiline strings in expressions (#4507, #3983), nested rsx! in vec! (#4106, #3591),
//! cfg-gated closures (#5523), or-patterns in if-let (#4268), block comments (#2751),
//! comments in every position (full-line, inline trailing, inside closure bodies,
//! at the start of if/else branches), deeply nested elements, and long attribute lists.

use arbitrary::{Result as ArbitraryResult, Unstructured};
use dioxus_autofmt::{IndentOptions, apply_formats, try_fmt_file};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunResult {
    ExhaustedInput,
    InvalidGeneratedInput,
    Checked,
}

struct FuzzCase {
    src: String,
    markers: Vec<String>,
}

struct Gen<'a> {
    input: Unstructured<'a>,
    /// Unique markers we expect to survive formatting (comment contents, string contents)
    markers: Vec<String>,
    next_marker: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            input: Unstructured::new(data),
            markers: Vec::new(),
            next_marker: 0,
        }
    }

    fn below(&mut self, n: usize) -> ArbitraryResult<usize> {
        self.input.int_in_range(0..=n - 1)
    }

    fn chance(&mut self, pct: u8) -> ArbitraryResult<bool> {
        self.input.ratio(pct, 100)
    }

    fn marker(&mut self, prefix: &str) -> String {
        let m = format!("{prefix}_{}", self.next_marker);
        self.next_marker += 1;
        self.markers.push(m.clone());
        m
    }

    fn ident(&mut self) -> ArbitraryResult<&'static str> {
        const IDENTS: &[&str] = &[
            "value", "items", "count", "name", "user", "data", "state", "config", "handler",
            "result",
        ];
        Ok(*self.input.choose(IDENTS)?)
    }

    fn element_name(&mut self) -> ArbitraryResult<&'static str> {
        const NAMES: &[&str] = &[
            "div", "span", "button", "input", "ul", "li", "p", "h1", "section", "article",
        ];
        Ok(*self.input.choose(NAMES)?)
    }

    fn component_name(&mut self) -> ArbitraryResult<&'static str> {
        const NAMES: &[&str] = &[
            "Card",
            "Header",
            "Footer",
            "ListItem",
            "icons::Icon",
            "ui::Button",
        ];
        Ok(*self.input.choose(NAMES)?)
    }

    fn text_contents(&mut self) -> ArbitraryResult<String> {
        let mut out = String::new();
        let words = 1 + self.below(6)?;
        for i in 0..words {
            if i > 0 {
                out.push(' ');
            }
            if self.chance(25)? {
                out.push_str(&format!("{{{}}}", self.ident()?));
            } else {
                out.push_str("word");
            }
        }
        Ok(out)
    }

    fn line_comment(&mut self) -> ArbitraryResult<String> {
        let marker = self.marker("comment");
        if self.chance(20)? {
            Ok(format!("/* {marker} */"))
        } else {
            Ok(format!("// {marker}"))
        }
    }

    fn simple_expr(&mut self) -> ArbitraryResult<String> {
        Ok(match self.below(6)? {
            0 => self.ident()?.to_string(),
            1 => format!("{}.clone()", self.ident()?),
            2 => format!("{}.len() + 1", self.ident()?),
            3 => format!("format!(\"{{}}\", {})", self.ident()?),
            4 => format!("{}.iter().count()", self.ident()?),
            _ => format!("Some({})", self.ident()?),
        })
    }

    fn condition(&mut self) -> ArbitraryResult<String> {
        Ok(match self.below(4)? {
            0 => format!("{}.is_some()", self.ident()?),
            1 => format!("{} > 0", self.ident()?),
            2 => format!("{}.is_empty()", self.ident()?),
            _ => format!("{} == {}", self.ident()?, self.ident()?),
        })
    }

    /// A multiline string literal whose interior indentation must be preserved verbatim (#4507)
    fn multiline_string(&mut self, indent: usize) -> ArbitraryResult<String> {
        let pad = "    ".repeat(indent);
        let m1 = self.marker("strline");
        let m2 = self.marker("strline");
        if self.chance(50)? {
            Ok(format!(
                "r#\"\n{pad}    {m1}();\n{pad}        {m2}();\n{pad}\"#"
            ))
        } else {
            Ok(format!("\"\n{pad}    {m1}\n{pad}        {m2}\n{pad}\""))
        }
    }

    /// Statements that go inside an event handler closure
    fn statements(&mut self, depth: usize, indent: usize) -> ArbitraryResult<String> {
        let pad = "    ".repeat(indent);
        let n = 1 + self.below(3)?;
        let mut out = String::new();
        for _ in 0..n {
            // comments inside event-handler closure bodies
            if self.chance(20)? {
                out.push_str(&format!("{pad}{}\n", self.line_comment()?));
            }
            match self.below(8)? {
                0 => out.push_str(&format!("{pad}{}.set(true);\n", self.ident()?)),
                1 => out.push_str(&format!(
                    "{pad}let {} = {};\n",
                    self.ident()?,
                    self.simple_expr()?
                )),
                2 => out.push_str(&format!("{pad}println!(\"{{}}\", {});\n", self.ident()?)),
                3 => {
                    // multiline string in a call - the #4507 / #3983 shape
                    out.push_str(&format!(
                        "{pad}let _ = document::eval(\n{pad}    {},\n{pad});\n",
                        self.multiline_string(indent + 1)?
                    ));
                }
                4 => {
                    // if let with an or-pattern - the #4268 shape
                    out.push_str(&format!(
                        "{pad}if let State::A({0}) | State::B({0}, _) = {1}() {{\n{pad}    println!(\"{{{0}}}\");\n{pad}}}\n",
                        self.ident()?,
                        self.ident()?
                    ));
                }
                5 if depth > 0 => {
                    // nested rsx! inside an expression
                    let inner = self.body_node(depth - 1, indent + 1)?;
                    out.push_str(&format!(
                        "{pad}let _ = rsx! {{\n{}{inner}\n{pad}}};\n",
                        "    ".repeat(indent + 1)
                    ));
                }
                6 => {
                    // cfg-gated let - the #5523 shape
                    out.push_str(&format!(
                        "{pad}#[cfg(target_os = \"android\")]\n{pad}let {} = {};\n",
                        self.ident()?,
                        self.simple_expr()?
                    ));
                }
                _ => out.push_str(&format!(
                    "{pad}if let Err(err) = {} {{\n{pad}    {}.set(format!(\"{{}}: {{}}\", \"err\", err));\n{pad}    return;\n{pad}}}\n",
                    self.simple_expr()?,
                    self.ident()?
                )),
            }
        }
        out.pop();
        Ok(out)
    }

    fn attribute(&mut self, depth: usize, indent: usize) -> ArbitraryResult<String> {
        Ok(match self.below(8)? {
            0 => format!("class: \"{}\"", self.text_contents()?),
            1 => format!("id: \"{}\"", self.marker("id")),
            2 => format!("\"data-x\": \"{}\"", self.text_contents()?),
            3 => self.ident()?.to_string(), // shorthand
            4 => {
                // avoid `value: value`, which the formatter intentionally collapses to shorthand
                let mut expr = self.simple_expr()?;
                if expr == "value" {
                    expr = "data".to_string();
                }
                format!("value: {expr}")
            }
            5 => format!(
                "class: if {} {{ \"{}\" }} else {{ \"{}\" }}",
                self.condition()?,
                self.text_contents()?,
                self.text_contents()?
            ),
            6 => {
                format!(
                    "onclick: move |_| {{\n{}\n{}}}",
                    self.statements(depth, indent + 1)?,
                    "    ".repeat(indent)
                )
            }
            _ => {
                let pad = "    ".repeat(indent + 1);
                format!(
                    "onmounted: move |_| async move {{\n{pad}{}\n{}}}",
                    self.statements(depth, indent + 1)?.trim_start(),
                    "    ".repeat(indent)
                )
            }
        })
    }

    fn expr_node(&mut self, depth: usize, indent: usize) -> ArbitraryResult<String> {
        let pad = "    ".repeat(indent);
        Ok(match self.below(5)? {
            0 => format!("{{{}}}", self.ident()?),
            1 => format!(
                "{{{}.iter().map(|i| rsx! {{ li {{ \"{{i}}\" }} }})}}",
                self.ident()?
            ),
            2 if depth > 0 => {
                // rsx in vec - the #4106 / #3591 shape
                let inner = self.body_node(depth - 1, indent + 2)?;
                format!(
                    "{{\n{pad}    vec![\n{pad}        rsx! {{\n{pad}            {inner}\n{pad}        }},\n{pad}        rsx! {{ \"{}\" }},\n{pad}    ]\n{pad}}}",
                    self.text_contents()?
                )
            }
            3 => format!(
                "{{{}.is_some().then(|| rsx! {{ span {{ \"{}\" }} }})}}",
                self.ident()?,
                self.text_contents()?
            ),
            _ => format!("{{Some({}).unwrap_or_default()}}", self.ident()?),
        })
    }

    /// Sometimes a full-line comment at the start of an if/else branch body
    fn branch_comment(&mut self, indent: usize) -> ArbitraryResult<String> {
        if self.chance(20)? {
            Ok(format!(
                "{}{}\n",
                "    ".repeat(indent),
                self.line_comment()?
            ))
        } else {
            Ok(String::new())
        }
    }

    fn body_node(&mut self, depth: usize, indent: usize) -> ArbitraryResult<String> {
        let pad = "    ".repeat(indent);
        let choice = if depth == 0 {
            self.below(2)?
        } else {
            self.below(7)?
        };

        Ok(match choice {
            // plain text
            0 => format!("\"{}\"", self.text_contents()?),
            // expression node
            1 => self.expr_node(depth, indent)?,
            // element with attrs and children
            2 | 3 => {
                let name = self.element_name()?;
                let mut out = format!("{name} {{\n");
                let n_attrs = self.below(4)?;
                for _ in 0..n_attrs {
                    if self.chance(15)? {
                        out.push_str(&format!("{pad}    {}\n", self.line_comment()?));
                    }
                    let attr = self.attribute(depth, indent + 1)?;
                    if self.chance(15)? {
                        // inline trailing comment after an attribute
                        let m = self.marker("trailing");
                        out.push_str(&format!("{pad}    {attr}, // {m}\n"));
                    } else {
                        out.push_str(&format!("{pad}    {attr},\n"));
                    }
                }
                let n_children = 1 + self.below(3)?;
                for _ in 0..n_children {
                    if self.chance(20)? {
                        out.push_str(&format!("{pad}    {}\n", self.line_comment()?));
                    }
                    let child = self.body_node(depth - 1, indent + 1)?;
                    if self.chance(15)? {
                        // inline trailing comment after a child node
                        let m = self.marker("trailing");
                        out.push_str(&format!("{pad}    {child} // {m}\n"));
                    } else {
                        out.push_str(&format!("{pad}    {child}\n"));
                    }
                }
                if self.chance(10)? {
                    out.push_str(&format!("{pad}    {}\n", self.line_comment()?));
                }
                out.push_str(&format!("{pad}}}"));
                out
            }
            // component
            4 => {
                let name = self.component_name()?;
                let mut out = format!("{name} {{\n");
                let n_fields = self.below(3)?;
                for _ in 0..n_fields {
                    out.push_str(&format!(
                        "{pad}    {},\n",
                        self.attribute(depth, indent + 1)?
                    ));
                }
                if self.chance(70)? {
                    out.push_str(&format!(
                        "{pad}    {}\n",
                        self.body_node(depth - 1, indent + 1)?
                    ));
                }
                out.push_str(&format!("{pad}}}"));
                out
            }
            // for loop
            5 => {
                let pat = if self.chance(50)? {
                    "item".to_string()
                } else {
                    "(i, item)".to_string()
                };
                let it = if pat.starts_with('(') {
                    format!("{}.iter().enumerate()", self.ident()?)
                } else {
                    self.ident()?.to_string()
                };
                format!(
                    "for {pat} in {it} {{\n{pad}    {}\n{pad}}}",
                    self.body_node(depth - 1, indent + 1)?
                )
            }
            // if chain
            _ => {
                let mut out = format!(
                    "if {} {{\n{}{pad}    {}\n{pad}}}",
                    self.condition()?,
                    self.branch_comment(indent + 1)?,
                    self.body_node(depth - 1, indent + 1)?
                );
                if self.chance(50)? {
                    out.push_str(&format!(
                        " else if {} {{\n{}{pad}    {}\n{pad}}}",
                        self.condition()?,
                        self.branch_comment(indent + 1)?,
                        self.body_node(depth - 1, indent + 1)?
                    ));
                }
                if self.chance(50)? {
                    out.push_str(&format!(
                        " else {{\n{}{pad}    {}\n{pad}}}",
                        self.branch_comment(indent + 1)?,
                        self.body_node(depth - 1, indent + 1)?
                    ));
                }
                out
            }
        })
    }

    fn file(&mut self) -> ArbitraryResult<FuzzCase> {
        // sometimes start the rsx! deeper in the file (#3591)
        let deep = self.chance(30)?;
        let base_indent = if deep { 2 } else { 1 };
        let pad = "    ".repeat(base_indent);

        let n_roots = 1 + self.below(3)?;
        let mut body = String::new();
        for _ in 0..n_roots {
            if self.chance(20)? {
                body.push_str(&format!("{pad}    {}\n", self.line_comment()?));
            }
            body.push_str(&format!(
                "{pad}    {}\n",
                self.body_node(2, base_indent + 1)?
            ));
        }

        let src = if deep {
            format!(
                "use dioxus::prelude::*;\n\nfn app() -> Element {{\n    let cb = move || {{\n        rsx! {{\n{body}        }}\n    }};\n    cb()\n}}\n"
            )
        } else {
            format!(
                "use dioxus::prelude::*;\n\nfn app() -> Element {{\n    rsx! {{\n{body}    }}\n}}\n"
            )
        };

        Ok(FuzzCase {
            src,
            markers: self.markers.clone(),
        })
    }
}

fn fmt(contents: &str) -> Result<String, String> {
    let parsed = syn::parse_file(contents).map_err(|e| format!("input does not parse: {e}"))?;
    let edits = try_fmt_file(contents, &parsed, IndentOptions::default())
        .map_err(|e| format!("fmt error: {e}"))?;
    Ok(apply_formats(contents, edits))
}

/// Normalized token representation: whitespace and commas stripped. Formatting may
/// legitimately add/remove commas and whitespace, but nothing else.
fn normalized_tokens(contents: &str) -> Result<String, String> {
    use quote::ToTokens;
    let file = syn::parse_file(contents).map_err(|e| format!("does not parse: {e}"))?;
    let mut out = file.to_token_stream().to_string();
    out.retain(|c| !c.is_whitespace() && c != ',');
    Ok(out)
}

fn check_invariants(src: &str, markers: &[String]) -> Result<(), String> {
    let once = std::panic::catch_unwind(|| fmt(src))
        .map_err(|_| "panic during first format".to_string())??;

    syn::parse_file(&once)
        .map_err(|e| format!("formatted output does not parse: {e}\n=== OUTPUT ===\n{once}"))?;

    let twice = std::panic::catch_unwind(|| fmt(&once))
        .map_err(|_| format!("panic during second format\n=== ONCE ===\n{once}"))??;

    if once != twice {
        let diff = once
            .lines()
            .zip(twice.lines())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("line {}:\n  once : {a}\n  twice: {b}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "not idempotent:\n{diff}\n=== ONCE ===\n{once}\n=== TWICE ===\n{twice}"
        ));
    }

    let tokens_in = normalized_tokens(src)?;
    let tokens_out = normalized_tokens(&once)
        .map_err(|e| format!("formatted output tokens: {e}\n=== OUTPUT ===\n{once}"))?;
    if tokens_in != tokens_out {
        return Err(format!(
            "tokens changed by formatting\n=== INPUT TOKENS ===\n{tokens_in}\n=== OUTPUT TOKENS ===\n{tokens_out}\n=== OUTPUT ===\n{once}"
        ));
    }

    for marker in markers {
        if !once.contains(marker.as_str()) {
            return Err(format!(
                "marker {marker:?} lost during formatting\n=== OUTPUT ===\n{once}"
            ));
        }
    }

    Ok(())
}

fn input_debug(data: &[u8]) -> String {
    const MAX_BYTES: usize = 128;
    if data.len() <= MAX_BYTES {
        format!("{data:?}")
    } else {
        format!("{:?} ... ({} bytes total)", &data[..MAX_BYTES], data.len())
    }
}

pub fn run_input(data: &[u8]) -> RunResult {
    let Ok(case) = Gen::new(data).file() else {
        return RunResult::ExhaustedInput;
    };

    // The generator can occasionally produce rsx that fails strict parsing
    // (e.g. expressions the rsx parser rejects). Skip those - we only care
    // about valid inputs.
    if fmt(&case.src).is_err() {
        return RunResult::InvalidGeneratedInput;
    }

    if let Err(err) = check_invariants(&case.src, &case.markers) {
        panic!(
            "{err}\n=== INPUT BYTES ===\n{data}\n=== INPUT ===\n{src}",
            data = input_debug(data),
            src = case.src
        );
    }

    RunResult::Checked
}
