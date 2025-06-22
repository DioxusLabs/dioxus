use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::{fs, path::Path};
use swc_common::comments::{CommentKind, Comments};
use swc_common::Spanned;
use swc_common::{comments::SingleThreadedComments, SourceMap, Span};
use swc_ecma_ast::{
    Decl, ExportDecl, ExportSpecifier, FnDecl, ModuleExportName, NamedExport, Param, Pat,
    VarDeclarator,
};
use swc_ecma_parser::EsSyntax;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitStr, Result, Token,
};

#[derive(Debug, Clone)]
enum ImportSpec {
    /// *
    All,
    /// {greeting, other_func}
    Named(Vec<Ident>),
    /// greeting
    Single(Ident),
}

struct UseJsInput {
    asset_path: LitStr,
    import_spec: ImportSpec,
}

impl Parse for UseJsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let asset_path: LitStr = input.parse()?;
        input.parse::<Token![::]>()?;

        let import_spec = if input.peek(Token![*]) {
            input.parse::<Token![*]>()?;
            ImportSpec::All
        } else if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            let mut functions = Vec::new();

            loop {
                let ident: Ident = content.parse()?;
                functions.push(ident);

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                    if content.is_empty() {
                        break;
                    }
                } else {
                    break;
                }
            }

            ImportSpec::Named(functions)
        } else {
            let ident: Ident = input.parse()?;
            ImportSpec::Single(ident)
        };

        Ok(UseJsInput {
            asset_path,
            import_spec,
        })
    }
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    /// If specified in the use declaration
    name_ident: Option<Ident>,
    params: Vec<String>,
    is_exported: bool,
    /// The stripped lines
    doc_comment: Vec<String>,
}

struct FunctionVisitor {
    functions: Vec<FunctionInfo>,
    comments: SingleThreadedComments,
}

impl FunctionVisitor {
    fn new(comments: SingleThreadedComments) -> Self {
        Self {
            functions: Vec::new(),
            comments,
        }
    }

    fn extract_doc_comment(&self, span: Span) -> Vec<String> {
        // Get leading comments for the span
        let leading_comment = self.comments.get_leading(span.lo());

        if let Some(comments) = leading_comment {
            let mut doc_lines = Vec::new();

            for comment in comments.iter() {
                let comment_text = &comment.text;
                match comment.kind {
                    // Handle `///`. `//` is already stripped
                    CommentKind::Line => {
                        if let Some(content) = comment_text.strip_prefix("/") {
                            let cleaned = content.trim_start();
                            doc_lines.push(cleaned.to_string());
                        }
                    }
                    // Handle `/*` `*/`. `/*` `*/` is already stripped
                    CommentKind::Block => {
                        for line in comment_text.lines() {
                            if let Some(cleaned) = line.trim_start().strip_prefix("*") {
                                doc_lines.push(cleaned.to_string());
                            }
                        }
                    }
                };
            }

            doc_lines
        } else {
            Vec::new()
        }
    }
}

fn function_params_to_names(params: &[Param]) -> Vec<String> {
    params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            if let Some(ident) = param.pat.as_ident() {
                ident.id.sym.to_string()
            } else {
                format!("arg{}", i)
            }
        })
        .collect()
}

fn function_pat_to_names(pats: &[Pat]) -> Vec<String> {
    pats.iter()
        .enumerate()
        .map(|(i, pat)| {
            if let Some(ident) = pat.as_ident() {
                ident.id.sym.to_string()
            } else {
                format!("arg{}", i)
            }
        })
        .collect()
}

impl Visit for FunctionVisitor {
    /// Visit function declarations: function foo() {}
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        let doc_comment = self.extract_doc_comment(node.span());

        self.functions.push(FunctionInfo {
            name: node.ident.sym.to_string(),
            name_ident: None,
            params: function_params_to_names(&node.function.params),
            is_exported: false,
            doc_comment,
        });
        node.visit_children_with(self);
    }

    /// Visit function expressions: const foo = function() {}
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        if let swc_ecma_ast::Pat::Ident(ident) = &node.name {
            if let Some(init) = &node.init {
                let doc_comment = self.extract_doc_comment(node.span());

                match &**init {
                    swc_ecma_ast::Expr::Fn(fn_expr) => {
                        self.functions.push(FunctionInfo {
                            name: ident.id.sym.to_string(),
                            name_ident: None,
                            params: function_params_to_names(&fn_expr.function.params),
                            is_exported: false,
                            doc_comment,
                        });
                    }
                    swc_ecma_ast::Expr::Arrow(arrow_fn) => {
                        self.functions.push(FunctionInfo {
                            name: ident.id.sym.to_string(),
                            name_ident: None,
                            params: function_pat_to_names(&arrow_fn.params),
                            is_exported: false,
                            doc_comment,
                        });
                    }
                    _ => {}
                }
            }
        }
        node.visit_children_with(self);
    }

    /// Visit export declarations: export function foo() {}
    fn visit_export_decl(&mut self, node: &ExportDecl) {
        if let Decl::Fn(fn_decl) = &node.decl {
            let doc_comment = self.extract_doc_comment(node.span());

            self.functions.push(FunctionInfo {
                name: fn_decl.ident.sym.to_string(),
                name_ident: None,
                params: function_params_to_names(&fn_decl.function.params),
                is_exported: true,
                doc_comment,
            });
        }
        node.visit_children_with(self);
    }

    /// Visit named exports: export { foo }
    fn visit_named_export(&mut self, node: &NamedExport) {
        for spec in &node.specifiers {
            if let ExportSpecifier::Named(named) = spec {
                let name = match &named.orig {
                    ModuleExportName::Ident(ident) => ident.sym.to_string(),
                    ModuleExportName::Str(str_lit) => str_lit.value.to_string(),
                };

                if let Some(func) = self.functions.iter_mut().find(|f| f.name == name) {
                    func.is_exported = true;
                }
            }
        }
        node.visit_children_with(self);
    }
}

fn parse_js_file(file_path: &Path) -> Result<Vec<FunctionInfo>> {
    let js_content = fs::read_to_string(file_path).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Could not read JavaScript file '{}': {}",
                file_path.display(),
                e
            ),
        )
    })?;

    let cm = SourceMap::default();
    let fm = cm.new_source_file(
        swc_common::FileName::Custom(file_path.display().to_string()).into(),
        js_content.clone(),
    );
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax::default()),
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    let module = parser.parse_module().map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Failed to parse JavaScript file '{}': {:?}",
                file_path.display(),
                e
            ),
        )
    })?;

    let mut visitor = FunctionVisitor::new(comments);
    module.visit_with(&mut visitor);

    // Functions are added twice for some reason
    visitor
        .functions
        .dedup_by(|e1, e2| e1.name.as_str() == e2.name.as_str());
    Ok(visitor.functions)
}

fn remove_function_info(name: &str, functions: &mut Vec<FunctionInfo>) -> Result<FunctionInfo> {
    if let Some(pos) = functions.iter().position(|f| f.name == name) {
        Ok(functions.remove(pos))
    } else {
        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("Function '{}' not found in JavaScript file", name),
        ))
    }
}

fn get_functions_to_generate(
    mut functions: Vec<FunctionInfo>,
    import_spec: ImportSpec,
) -> Result<Vec<FunctionInfo>> {
    match import_spec {
        ImportSpec::All => Ok(functions),
        ImportSpec::Single(name) => {
            let mut func = remove_function_info(name.to_string().as_str(), &mut functions)?;
            func.name_ident.replace(name);
            Ok(vec![func])
        }
        ImportSpec::Named(names) => {
            let mut result = Vec::new();
            for name in names {
                let mut func = remove_function_info(name.to_string().as_str(), &mut functions)?;
                func.name_ident.replace(name);
                result.push(func);
            }
            Ok(result)
        }
    }
}

fn generate_function_wrapper(func: &FunctionInfo, asset_path: &LitStr) -> TokenStream2 {
    let send_calls: Vec<TokenStream2> = func
        .params
        .iter()
        .map(|param| {
            let param = format_ident!("{}", param);
            quote! {
                eval.send(#param)?;
            }
        })
        .collect();

    let js_func_name = &func.name;
    let mut js_format = format!(r#"const {{{{ {js_func_name} }}}} = await import("{{}}");"#);
    for param in func.params.iter() {
        js_format.push_str(&format!("\nlet {} = await dioxus.recv();", param));
    }
    js_format.push_str(&format!("\nreturn {}(", js_func_name));
    for (i, param) in func.params.iter().enumerate() {
        if i > 0 {
            js_format.push_str(", ");
        }
        js_format.push_str(param.as_str());
    }
    js_format.push_str(");");

    let param_types: Vec<_> = func
        .params
        .iter()
        .map(|param| {
            let param = format_ident!("{}", param);
            quote! { #param: impl serde::Serialize }
        })
        .collect();

    // Generate documentation comment if available - preserve original JSDoc format
    let doc_comment = if func.doc_comment.is_empty() {
        quote! {}
    } else {
        let doc_lines: Vec<_> = func
            .doc_comment
            .iter()
            .map(|line| quote! { #[doc = #line] })
            .collect();
        quote! { #(#doc_lines)* }
    };

    let func_name = func
        .name_ident
        .clone()
        // Can not exist if `::*`
        .unwrap_or_else(|| Ident::new(func.name.as_str(), proc_macro2::Span::call_site()));
    quote! {
        #doc_comment
        pub async fn #func_name(#(#param_types),*) -> Result<serde_json::Value, document::EvalError> {
            const MODULE: Asset = asset!(#asset_path);
            let js = format!(#js_format, MODULE);
            let eval = document::eval(js.as_str());
            #(#send_calls)*
            eval.await
        }
    }
}

/// A macro to create rust binding to javascript functions.
///```rust,ignore
/// use dioxus::prelude::*;
///
/// // Generate the greeting function at compile time
/// use_js!("examples/assets/example.js"::greeting);
///
///  // Or generate multiple functions:
///  // use_js!("examples/assets/example.js"::{greeting, add});
///
///  // Or generate all exported functions:
///  // use_js!("examples/assets/example.js"::*);
///
/// fn main() {
///     launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     let future = use_resource(|| async move {
///         let from = "dave";
///         let to = "john";
///
///         // Now we can call the generated function directly!
///         let greeting_result = greeting(from, to)
///             .await
///             .map_err(Box::<dyn std::error::Error>::from)?;
///         let greeting: String =
///             serde_json::from_value(greeting_result).map_err(Box::<dyn std::error::Error>::from)?;
///         Ok::<String, Box<dyn std::error::Error>>(greeting)
///     });
///
///     rsx!(
///         div {
///             h1 { "Dioxus `use_js!` macro example!" }
///             {
///                 match &*future.read() {
///                     Some(Ok(greeting)) => rsx! {
///                         p { "Greeting from JavaScript: {greeting}" }
///                     },
///                     Some(Err(e)) => rsx! {
///                         p { "Error: {e}" }
///                     },
///                     None => rsx! {
///                         p { "Running js..." }
///                     },
///                 }
///             }
///         }
///     )
/// }
/// ```
#[proc_macro]
pub fn use_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as UseJsInput);

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return TokenStream::from(
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "CARGO_MANIFEST_DIR environment variable not found",
                )
                .to_compile_error(),
            );
        }
    };

    let asset_path = &input.asset_path;
    let js_file_path = std::path::Path::new(&manifest_dir).join(asset_path.value());

    let all_functions = match parse_js_file(&js_file_path) {
        Ok(funcs) => funcs,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let import_spec = input.import_spec;
    let functions_to_generate = match get_functions_to_generate(all_functions, import_spec) {
        Ok(funcs) => funcs,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let function_wrappers: Vec<TokenStream2> = functions_to_generate
        .iter()
        .map(|func| generate_function_wrapper(func, asset_path))
        .collect();

    let expanded = quote! {
        #(#function_wrappers)*
    };

    TokenStream::from(expanded)
}
