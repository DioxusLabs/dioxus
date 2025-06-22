use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::sync::Arc;
use std::{fs, path::Path};
use swc_common::SourceMap;
use swc_ecma_ast::{
    Decl, ExportDecl, ExportSpecifier, FnDecl, ModuleExportName, NamedExport, VarDeclarator,
};
use swc_ecma_parser::EsSyntax;
use swc_ecma_parser::{Parser, StringInput, Syntax, lexer::Lexer};
use swc_ecma_visit::{Visit, VisitWith};
use syn::{
    Expr, ExprCall, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct CallJsInput {
    asset_path: LitStr,
    function_call: ExprCall,
}

impl Parse for CallJsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let asset_path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;

        let function_call: ExprCall = input.parse()?;

        Ok(CallJsInput {
            asset_path,
            function_call,
        })
    }
}

fn extract_function_name(call: &ExprCall) -> Result<String> {
    match &*call.func {
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                Ok(ident.to_string())
            } else {
                Err(syn::Error::new_spanned(
                    &path.path,
                    "Function call must be a simple identifier",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            &call.func,
            "Function call must be a simple identifier",
        )),
    }
}

#[derive(Debug)]
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

fn validate_function_call(
    functions: &[FunctionInfo],
    function_name: &str,
    arg_count: usize,
) -> Result<()> {
    let function = functions
        .iter()
        .find(|f| f.name == function_name)
        .ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Function '{}' not found in JavaScript file", function_name),
            )
        })?;

    if !function.is_exported {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Function '{}' is not exported from the JavaScript module",
                function_name
            ),
        ));
    }

    if function.param_count != arg_count {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Function '{}' expects {} arguments, but {} were provided",
                function_name, function.param_count, arg_count
            ),
        ));
    }

    Ok(())
}

#[proc_macro]
pub fn call_js(input: TokenStream) -> TokenStream {
    // parse
    let input = parse_macro_input!(input as CallJsInput);

    let asset_path = &input.asset_path;
    let function_call = &input.function_call;

    let function_name = match extract_function_name(function_call) {
        Ok(name) => name,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    // validate js call
    let arg_count = function_call.args.len();
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
    let functions = match parse_js_file(&js_file_path) {
        Ok(funcs) => funcs,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    if let Err(e) = validate_function_call(&functions, &function_name, arg_count) {
        return TokenStream::from(e.to_compile_error());
    }

    // expand
    let send_calls: Vec<TokenStream2> = function_call
        .args
        .iter()
        .map(|arg| quote! { eval.send(#arg)?; })
        .collect();

    let mut js_format = format!(r#"const {{{{ {function_name} }}}} = await import("{{}}");"#,);
    for i in 0..arg_count {
        js_format.push_str(&format!("\nlet arg{} = await dioxus.recv();", i));
    }
    js_format.push_str(&format!("\nreturn {}(", function_name));
    for i in 0..arg_count {
        if i > 0 {
            js_format.push_str(", ");
        }
        js_format.push_str(&format!("arg{}", i));
    }
    js_format.push_str(");");

    let expanded = quote! {
        async move {
            const MODULE: Asset = asset!(#asset_path);
            let js = format!(#js_format, MODULE);
            let eval = document::eval(js.as_str());
            #(#send_calls)*
            eval.await
        }
    };

    TokenStream::from(expanded)
}
