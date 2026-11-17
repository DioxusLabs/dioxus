use std::path::Path;

use anyhow::Context;
use manganis_core::JsAssetOptions;

use crate::opt::hash::hash_file_contents;
pub(crate) fn process_js(
    js_options: &JsAssetOptions,
    source: &Path,
    output_path: &Path,
    esbuild_path: Option<&Path>,
) -> anyhow::Result<()> {
    if js_options.minified() || js_options.is_module() {
        if let Some(esbuild) = esbuild_path {
            match run_esbuild(esbuild, source, output_path, js_options) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    tracing::error!(
                        "Failed to process JS with esbuild. Falling back to copy: {err}"
                    );
                }
            }
        } else {
            tracing::warn!("esbuild binary path not set. Copying JS without processing.");
        }
    }

    // Fallback / no minification: copy unprocessed
    let mut source_file = std::fs::File::open(source)?;
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output_path)?);
    std::io::copy(&mut source_file, &mut writer).with_context(|| {
        format!(
            "Failed to write JS to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

/// Run esbuild to minify a JavaScript file in place.
///
/// When `is_module` is true, the file is treated as ES module input:
/// `--bundle --format=esm` inlines local relative imports (notably the
/// `snippets/` folder that wasm-bindgen emits for `#[wasm_bindgen(inline_js)]`
/// / `module = "…"`) into a single ESM file. `http://` and `https://` imports
/// are marked external so URL-based module loading (e.g. CDN-hosted ESM,
/// firebase) is left for the browser to resolve at runtime. The consuming
/// `<script>` tag is expected to be `type="module"`.
///
/// When `is_module` is false, only `--minify` is passed and esbuild preserves
/// the input's format verbatim — a classic IIFE/UMD script stays a classic
/// script with no wrapper added.
fn run_esbuild(
    esbuild: &Path,
    source: &Path,
    output_path: &Path,
    js_options: &JsAssetOptions,
) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(esbuild);
    cmd.arg(source);
    cmd.arg(format!("--outfile={}", output_path.display()));
    cmd.arg("--log-level=warning");

    if js_options.minified() {
        cmd.arg("--minify");
    }

    if lexer::js_is_module(js_options, source) {
        cmd.arg("--bundle");
        cmd.arg("--format=esm");
        // Don't try to resolve URL-based imports at build time — let the
        // browser fetch them at runtime. Without these externals, esbuild
        // errors out on patterns like `import x from "https://cdn/lib.js"`.
        cmd.arg("--external:https://*");
        cmd.arg("--external:http://*");
    }

    tracing::debug!("Running esbuild: {:?}", cmd);

    let output = cmd.output().context("Failed to run esbuild")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("esbuild failed: {stderr}");
    }

    Ok(())
}

pub(crate) fn hash_js(
    _js_options: &JsAssetOptions,
    source: &Path,
    hasher: &mut impl std::hash::Hasher,
) -> anyhow::Result<()> {
    hash_file_contents(source, hasher)
}

pub use lexer::*;
mod lexer {
    //! Detect whether a JavaScript source file uses ES module syntax.
    //!
    //! This is a token-level scanner inspired by Guy Bedford's `es-module-lexer`
    //! (the same heuristic used by Vite, Rollup, and Node's `--experimental-detect-module`):
    //! skip strings, comments, template literals, and regex literals, then look for
    //! a top-level `import` declaration, a top-level `export` declaration, or an
    //! `import.meta` reference. Dynamic `import(...)` and `require(...)` do not
    //! count as module syntax — they are valid in classic scripts and CommonJS.
    //!
    //! This is intentionally a tokenizer, not a parser. We don't build an AST. The
    //! goal is a fast, dependency-free boolean decision for the asset pipeline.
    //!
    //! Edge cases on the regex/division ambiguity are biased toward false negatives
    //! (treating an ESM file as classic) rather than false positives (treating a
    //! classic file as ESM), so a misclassified file gets the "copy verbatim" path
    //! instead of the "wrap as module" path that broke the issue #5512 case.

    use std::path::Path;

    use manganis_core::JsAssetOptions;

    /// Resolve whether a JS asset should be treated as an ES module.
    ///
    /// Resolution order:
    /// 1. Explicit `with_module(true)` always wins.
    /// 2. File extension fast-path: `.mjs` is always ESM, `.cjs` is always classic.
    /// 3. Otherwise, scan the source and look for top-level `import`/`export` or
    ///    `import.meta`. If the file can't be read, treat it as classic.
    pub fn js_is_module(js_options: &JsAssetOptions, source: &Path) -> bool {
        if js_options.is_module() {
            return true;
        }
        match source.extension().and_then(|ext| ext.to_str()) {
            Some("mjs") => return true,
            Some("cjs") => return false,
            _ => {}
        }
        match std::fs::read_to_string(source) {
            Ok(content) => has_module_syntax(&content),
            Err(err) => {
                tracing::debug!(
                    "Failed to read JS source for module detection ({}): {err}. Treating as classic.",
                    source.display()
                );
                false
            }
        }
    }

    /// Detect whether `source` uses ES module syntax.
    ///
    /// Returns `true` if and only if the source contains at least one of:
    /// - a top-level `import` declaration (`import x from '…'`, `import '…'`, etc.)
    /// - a top-level `export` declaration (`export const`, `export default`, …)
    /// - an `import.meta` reference at any nesting level
    ///
    /// Dynamic `import(…)`, `require(…)`, and `module.exports` are *not* module syntax.
    pub fn has_module_syntax(source: &str) -> bool {
        let bytes = source.as_bytes();
        let mut i = 0;
        let mut brace_depth: i32 = 0;
        // True when the next `/` should be parsed as the start of a regex literal
        // rather than the division operator. Initially true (start of file).
        let mut expect_expr = true;

        while i < bytes.len() {
            let c = bytes[i];

            match c {
                // Whitespace
                b' ' | b'\t' | b'\n' | b'\r' => {
                    i += 1;
                }

                // Comments and the regex/division fork
                b'/' => match bytes.get(i + 1).copied() {
                    Some(b'/') => {
                        i = skip_line_comment(bytes, i);
                    }
                    Some(b'*') => {
                        i = skip_block_comment(bytes, i);
                    }
                    _ => {
                        if expect_expr {
                            i = skip_regex(bytes, i);
                            expect_expr = false;
                        } else {
                            i += 1;
                            expect_expr = true;
                        }
                    }
                },

                b'\'' | b'"' => {
                    i = skip_string(bytes, i, c);
                    expect_expr = false;
                }

                b'`' => {
                    i = skip_template(bytes, i);
                    expect_expr = false;
                }

                b'{' => {
                    brace_depth += 1;
                    i += 1;
                    expect_expr = true;
                }

                b'}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    i += 1;
                    // After `}` we conservatively assume operator position. This
                    // misclassifies regex literals at statement-start (rare) as
                    // division, which is the safer direction per the false-negative
                    // bias.
                    expect_expr = false;
                }

                // Open-grouping punctuators introduce an expression position.
                b'(' | b'[' => {
                    i += 1;
                    expect_expr = true;
                }

                // Close-grouping punctuators end an expression.
                b')' | b']' => {
                    i += 1;
                    expect_expr = false;
                }

                // Operators that take an operand on the right.
                b'=' | b',' | b';' | b'!' | b'~' | b'?' | b':' | b'<' | b'>' | b'&' | b'|'
                | b'^' | b'%' | b'*' | b'+' | b'-' => {
                    i += 1;
                    expect_expr = true;
                }

                // Member access — next token is an identifier, not an expression.
                b'.' => {
                    i += 1;
                    expect_expr = false;
                }

                c if is_identifier_start(c) => {
                    let start = i;
                    while i < bytes.len() && is_identifier_continue(bytes[i]) {
                        i += 1;
                    }
                    let ident = &bytes[start..i];

                    match ident {
                        b"import" => {
                            let after = skip_ws_and_comments(bytes, i);
                            match bytes.get(after).copied() {
                                Some(b'(') => {
                                    // Dynamic import — not module syntax.
                                    expect_expr = false;
                                }
                                Some(b'.') => {
                                    // `import.meta` — module syntax at any depth.
                                    return true;
                                }
                                _ => {
                                    // Static `import` declaration. Only legal at
                                    // module top level; treat top-level occurrences
                                    // as the ESM signal and ignore anything inside
                                    // braces (where it'd be a parse error anyway).
                                    if brace_depth == 0 {
                                        return true;
                                    }
                                    expect_expr = false;
                                }
                            }
                        }
                        b"export" => {
                            if brace_depth == 0 {
                                return true;
                            }
                            expect_expr = false;
                        }
                        // Keywords that introduce an expression.
                        b"return" | b"throw" | b"typeof" | b"void" | b"delete" | b"new" | b"in"
                        | b"instanceof" | b"await" | b"yield" | b"case" | b"else" | b"do" => {
                            expect_expr = true;
                        }
                        _ => {
                            expect_expr = false;
                        }
                    }
                }

                c if c.is_ascii_digit() => {
                    i = skip_number(bytes, i);
                    expect_expr = false;
                }

                // Anything else: advance one byte. This handles non-ASCII identifiers
                // conservatively (treated as identifier-ish: expect_expr unchanged).
                _ => {
                    i += 1;
                }
            }
        }

        false
    }

    fn skip_line_comment(bytes: &[u8], mut i: usize) -> usize {
        i += 2;
        while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
            i += 1;
        }
        i
    }

    fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
        i += 2;
        while i + 1 < bytes.len() {
            if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                return i + 2;
            }
            i += 1;
        }
        bytes.len()
    }

    fn skip_string(bytes: &[u8], mut i: usize, quote: u8) -> usize {
        i += 1; // opening quote
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if i + 1 < bytes.len() => i += 2,
                c if c == quote => return i + 1,
                // Strings can't span unescaped newlines; bail out at one for safety
                // even though that's technically a parse error.
                b'\n' => return i + 1,
                _ => i += 1,
            }
        }
        bytes.len()
    }

    fn skip_template(bytes: &[u8], mut i: usize) -> usize {
        i += 1; // opening backtick
        while i < bytes.len() {
            match bytes[i] {
                b'`' => return i + 1,
                b'\\' if i + 1 < bytes.len() => i += 2,
                b'$' if bytes.get(i + 1) == Some(&b'{') => {
                    i += 2;
                    i = skip_template_expression(bytes, i);
                }
                _ => i += 1,
            }
        }
        bytes.len()
    }

    /// Skip the expression inside a template's `${ … }`. Returns the position
    /// just past the closing `}`. Recurses into nested templates and skips
    /// strings/comments so that braces inside them don't affect the depth count.
    fn skip_template_expression(bytes: &[u8], mut i: usize) -> usize {
        let mut depth: i32 = 1;
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'{' => {
                    depth += 1;
                    i += 1;
                }
                b'}' => {
                    depth -= 1;
                    i += 1;
                }
                b'`' => {
                    i = skip_template(bytes, i);
                }
                b'\'' | b'"' => {
                    i = skip_string(bytes, i, bytes[i]);
                }
                b'/' => match bytes.get(i + 1).copied() {
                    Some(b'/') => i = skip_line_comment(bytes, i),
                    Some(b'*') => i = skip_block_comment(bytes, i),
                    _ => i += 1,
                },
                _ => i += 1,
            }
        }
        i
    }

    fn skip_regex(bytes: &[u8], mut i: usize) -> usize {
        i += 1; // opening slash
        let mut in_class = false;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if i + 1 < bytes.len() => i += 2,
                b'[' => {
                    in_class = true;
                    i += 1;
                }
                b']' if in_class => {
                    in_class = false;
                    i += 1;
                }
                b'/' if !in_class => {
                    i += 1;
                    // Skip flags
                    while i < bytes.len() && is_identifier_continue(bytes[i]) {
                        i += 1;
                    }
                    return i;
                }
                // Unterminated regex — bail.
                b'\n' => return i,
                _ => i += 1,
            }
        }
        i
    }

    fn skip_ws_and_comments(bytes: &[u8], mut i: usize) -> usize {
        loop {
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
                i += 1;
            }
            if i + 1 < bytes.len() && bytes[i] == b'/' {
                match bytes[i + 1] {
                    b'/' => {
                        i = skip_line_comment(bytes, i);
                        continue;
                    }
                    b'*' => {
                        i = skip_block_comment(bytes, i);
                        continue;
                    }
                    _ => {}
                }
            }
            return i;
        }
    }

    fn skip_number(bytes: &[u8], start: usize) -> usize {
        // Crude: consume identifier-continue chars after the leading digit. This
        // covers integer/float/hex/binary/octal/bigint and exponent forms well
        // enough for the only purpose this scanner has — advancing past numeric
        // literals so we don't misread their characters as keywords.
        let mut i = start + 1;
        while i < bytes.len() {
            let c = bytes[i];
            if (is_identifier_continue(c) || c == b'.')
                || ((c == b'+' || c == b'-') && i > start && matches!(bytes[i - 1], b'e' | b'E'))
            {
                i += 1;
            } else {
                break;
            }
        }
        i
    }

    fn is_identifier_start(c: u8) -> bool {
        c.is_ascii_alphabetic() || c == b'_' || c == b'$'
    }

    fn is_identifier_continue(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
    }

    #[cfg(test)]
    mod tests {
        use super::has_module_syntax;

        // --- Should detect ES module syntax ---

        #[test]
        fn detects_static_import_side_effect() {
            assert!(has_module_syntax("import 'foo';"));
        }

        #[test]
        fn detects_static_import_default() {
            assert!(has_module_syntax("import foo from 'bar';"));
        }

        #[test]
        fn detects_static_import_named() {
            assert!(has_module_syntax("import { foo } from 'bar';"));
        }

        #[test]
        fn detects_static_import_namespace() {
            assert!(has_module_syntax("import * as foo from 'bar';"));
        }

        #[test]
        fn detects_export_default() {
            assert!(has_module_syntax("export default function () {}"));
        }

        #[test]
        fn detects_export_named() {
            assert!(has_module_syntax("export { foo };"));
        }

        #[test]
        fn detects_export_const() {
            assert!(has_module_syntax("export const foo = 1;"));
        }

        #[test]
        fn detects_export_function() {
            assert!(has_module_syntax("export function foo() {}"));
        }

        #[test]
        fn detects_export_class() {
            assert!(has_module_syntax("export class Foo {}"));
        }

        #[test]
        fn detects_import_meta() {
            assert!(has_module_syntax("console.log(import.meta.url);"));
        }

        #[test]
        fn detects_empty_export() {
            assert!(has_module_syntax("export {};"));
        }

        #[test]
        fn detects_export_after_block_comment() {
            assert!(has_module_syntax(
                "/* leading comment */\nexport const x = 1;"
            ));
        }

        #[test]
        fn detects_export_after_line_comment() {
            assert!(has_module_syntax("// leading\nexport const x = 1;"));
        }

        #[test]
        fn detects_url_import() {
            // The firebase-style use case from issue #3748.
            assert!(has_module_syntax(
                r#"import { initializeApp } from "https://www.gstatic.com/firebasejs/11.1.0/firebase-app.js";"#
            ));
        }

        // --- Should NOT detect (classic / CJS / etc.) ---

        #[test]
        fn rejects_empty_string() {
            assert!(!has_module_syntax(""));
        }

        #[test]
        fn rejects_dynamic_import_only() {
            assert!(!has_module_syntax("import('foo');"));
        }

        #[test]
        fn rejects_dynamic_import_in_assignment() {
            assert!(!has_module_syntax("var x = import('foo');"));
        }

        #[test]
        fn rejects_dynamic_import_inside_function() {
            assert!(!has_module_syntax("function foo() { import('x'); }"));
        }

        #[test]
        fn rejects_require() {
            assert!(!has_module_syntax("const foo = require('foo');"));
        }

        #[test]
        fn rejects_module_exports() {
            assert!(!has_module_syntax("module.exports = foo;"));
        }

        #[test]
        fn rejects_classic_script() {
            assert!(!has_module_syntax(
                "var x = 1; window.foo = x; console.log(x);"
            ));
        }

        #[test]
        fn rejects_import_inside_line_comment() {
            assert!(!has_module_syntax("// import foo from 'bar'\nvar x;"));
        }

        #[test]
        fn rejects_export_inside_block_comment() {
            assert!(!has_module_syntax("/* export default foo */\nvar x;"));
        }

        #[test]
        fn rejects_import_inside_string_double() {
            assert!(!has_module_syntax(r#"var x = "import foo from bar";"#));
        }

        #[test]
        fn rejects_import_inside_string_single() {
            assert!(!has_module_syntax("var x = 'import foo from bar';"));
        }

        #[test]
        fn rejects_import_inside_template() {
            assert!(!has_module_syntax(
                "var x = `export default ${y}`; var z = 1;"
            ));
        }

        #[test]
        fn rejects_import_inside_regex() {
            assert!(!has_module_syntax("var x = /import foo/.test('foo');"));
        }

        #[test]
        fn rejects_export_inside_regex_class() {
            assert!(!has_module_syntax("var x = /[export]/i.test('foo');"));
        }

        #[test]
        fn rejects_method_named_import() {
            // `obj.import('bar')` — `.import` is a property access, not a keyword.
            assert!(!has_module_syntax("foo.import('bar');"));
        }

        #[test]
        fn rejects_identifier_starting_with_import() {
            // `importFoo` is a regular identifier; the keyword check requires a
            // word boundary at the end as well.
            assert!(!has_module_syntax("var importFoo = 1; importFoo();"));
        }

        #[test]
        fn rejects_umd_wrapper() {
            // The SweetAlert2-shaped UMD bundle from issue #5512.
            let umd = r#"(function (root, factory) {
            if (typeof exports === 'object' && typeof module === 'object')
                module.exports = factory();
            else if (typeof define === 'function' && define.amd)
                define(factory);
            else
                root.Sweetalert2 = factory();
        })(this, function () { return {}; });"#;
            assert!(!has_module_syntax(umd));
        }

        #[test]
        fn rejects_iife() {
            assert!(!has_module_syntax(
                "(function () { window.foo = 'bar'; })();"
            ));
        }

        #[test]
        fn handles_template_with_brace_in_string() {
            // Make sure the `}` inside the inner string doesn't end the template
            // expression early.
            assert!(!has_module_syntax("var x = `${ '}}}' }`; var y = 1;"));
        }

        #[test]
        fn handles_nested_template() {
            assert!(!has_module_syntax("var x = `outer ${`inner ${1}`} done`;"));
        }

        #[test]
        fn detects_export_after_iife() {
            // An ESM file that begins with an IIFE and then has an export.
            assert!(has_module_syntax(
                "(function () { var x = 1; })();\nexport const y = 2;"
            ));
        }
    }
}
