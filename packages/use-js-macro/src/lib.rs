use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::sync::Arc;
use std::{fs, path::Path};
use swc_common::SourceMap;
use swc_ecma_ast::{
    Decl, ExportDecl, ExportSpecifier, FnDecl, ModuleExportName, NamedExport, VarDeclarator,
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
    Named(Vec<String>),
    /// greeting
    Single(String),
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
                functions.push(ident.to_string());

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
            ImportSpec::Single(ident.to_string())
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
    param_count: usize,
    is_exported: bool,
}

struct FunctionVisitor {
    functions: Vec<FunctionInfo>,
}

impl FunctionVisitor {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

impl Visit for FunctionVisitor {
    /// Visit function declarations: function foo() {}
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        self.functions.push(FunctionInfo {
            name: node.ident.sym.to_string(),
            param_count: node.function.params.len(),
            is_exported: false,
        });
        node.visit_children_with(self);
    }

    /// Visit function expressions: const foo = function() {}
    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        if let swc_ecma_ast::Pat::Ident(ident) = &node.name {
            if let Some(init) = &node.init {
                match &**init {
                    swc_ecma_ast::Expr::Fn(fn_expr) => {
                        self.functions.push(FunctionInfo {
                            name: ident.id.sym.to_string(),
                            param_count: fn_expr.function.params.len(),
                            is_exported: false,
                        });
                    }
                    swc_ecma_ast::Expr::Arrow(arrow_fn) => {
                        self.functions.push(FunctionInfo {
                            name: ident.id.sym.to_string(),
                            param_count: arrow_fn.params.len(),
                            is_exported: false,
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
        match &node.decl {
            Decl::Fn(fn_decl) => {
                self.functions.push(FunctionInfo {
                    name: fn_decl.ident.sym.to_string(),
                    param_count: fn_decl.function.params.len(),
                    is_exported: true,
                });
            }
            _ => {}
        }
        node.visit_children_with(self);
    }

    /// Visit named exports: export { foo }
    fn visit_named_export(&mut self, node: &NamedExport) {
        for spec in &node.specifiers {
            match spec {
                ExportSpecifier::Named(named) => {
                    let name = match &named.orig {
                        ModuleExportName::Ident(ident) => ident.sym.to_string(),
                        ModuleExportName::Str(str_lit) => str_lit.value.to_string(),
                    };

                    if let Some(func) = self.functions.iter_mut().find(|f| f.name == name) {
                        func.is_exported = true;
                    }
                }
                _ => {}
            }
        }
        node.visit_children_with(self);
    }
}

fn parse_js_file(file_path: &Path) -> Result<Vec<FunctionInfo>> {
    let js_content = fs::read_to_string(&file_path).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Could not read JavaScript file '{}': {}",
                file_path.display(),
                e
            ),
        )
    })?;

    let cm = Arc::new(SourceMap::default());
    let fm = cm.new_source_file(
        swc_common::FileName::Custom(file_path.display().to_string()).into(),
        js_content.clone(),
    );

    let lexer = Lexer::new(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        None,
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

    let mut visitor = FunctionVisitor::new();
    module.visit_with(&mut visitor);

    Ok(visitor.functions)
}

fn get_functions_to_generate(
    functions: &[FunctionInfo],
    import_spec: &ImportSpec,
) -> Result<Vec<FunctionInfo>> {
    let exported_functions: Vec<_> = functions
        .iter()
        .filter(|f| f.is_exported)
        .cloned()
        .collect();

    match import_spec {
        ImportSpec::All => Ok(exported_functions),
        ImportSpec::Single(name) => {
            let func = exported_functions
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| {
                    syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!(
                            "Function '{}' not found or not exported in JavaScript file",
                            name
                        ),
                    )
                })?;
            Ok(vec![func.clone()])
        }
        ImportSpec::Named(names) => {
            let mut result = Vec::new();
            for name in names {
                let func = exported_functions
                    .iter()
                    .find(|f| &f.name == name)
                    .ok_or_else(|| {
                        syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "Function '{}' not found or not exported in JavaScript file",
                                name
                            ),
                        )
                    })?;
                result.push(func.clone());
            }
            Ok(result)
        }
    }
}

fn generate_function_wrapper(func: &FunctionInfo, asset_path: &LitStr) -> TokenStream2 {
    let func_name = format_ident!("{}", func.name);
    let js_func_name = &func.name;

    let params: Vec<_> = (0..func.param_count)
        .map(|i| format_ident!("arg{}", i))
        .collect();

    let send_calls: Vec<TokenStream2> = params
        .iter()
        .map(|param| quote! { eval.send(#param)?; })
        .collect();

    let mut js_format = format!(r#"const {{{{ {js_func_name} }}}} = await import("{{}}");"#);
    for i in 0..func.param_count {
        js_format.push_str(&format!("\nlet arg{} = await dioxus.recv();", i));
    }
    js_format.push_str(&format!("\nreturn {}(", js_func_name));
    for i in 0..func.param_count {
        if i > 0 {
            js_format.push_str(", ");
        }
        js_format.push_str(&format!("arg{}", i));
    }
    js_format.push_str(");");

    let param_types: Vec<_> = (0..func.param_count)
        .map(|i| {
            let param = format_ident!("arg{}", i);
            quote! { #param: impl serde::Serialize }
        })
        .collect();

    quote! {
        pub async fn #func_name(#(#param_types),*) -> Result<serde_json::Value, document::EvalError> {
            const MODULE: Asset = asset!(#asset_path);
            let js = format!(#js_format, MODULE);
            let eval = document::eval(js.as_str());
            #(#send_calls)*
            eval.await
        }
    }
}

#[proc_macro]
pub fn use_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as UseJsInput);

    let asset_path = &input.asset_path;
    let import_spec = &input.import_spec;

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

    let js_file_path = std::path::Path::new(&manifest_dir).join(asset_path.value());

    let all_functions = match parse_js_file(&js_file_path) {
        Ok(funcs) => funcs,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let functions_to_generate = match get_functions_to_generate(&all_functions, import_spec) {
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
