//! FFI bridge macro for native plugin interop.
//!
//! This module implements the `#[ffi]` attribute macro that generates direct FFI bindings
//! between Rust and native platforms (Swift/Kotlin).

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    ForeignItem, ForeignItemFn, Ident, ItemForeignMod, LitStr, Pat, ReturnType, Type,
};

/// The foreign ABI being targeted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForeignAbi {
    /// Swift (iOS/macOS)
    Swift,
    /// Kotlin (Android)
    Kotlin,
}

/// A foreign type declaration (`type Foo;`)
#[derive(Debug, Clone)]
pub struct ForeignTypeDecl {
    pub name: Ident,
}

/// A parsed foreign type in function signatures
#[derive(Debug, Clone)]
pub enum ForeignType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    StrRef,
    Option(Box<ForeignType>),
    Result(Box<ForeignType>, Box<ForeignType>),
    OpaqueRef(Ident),
    Unit,
}

impl ForeignType {
    /// Parse a Rust type into a ForeignType
    fn from_type(ty: &Type) -> syn::Result<Self> {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;
                if path.segments.len() == 1 {
                    let segment = &path.segments[0];
                    let ident = segment.ident.to_string();
                    match ident.as_str() {
                        "bool" => return Ok(ForeignType::Bool),
                        "i8" => return Ok(ForeignType::I8),
                        "i16" => return Ok(ForeignType::I16),
                        "i32" => return Ok(ForeignType::I32),
                        "i64" => return Ok(ForeignType::I64),
                        "u8" => return Ok(ForeignType::U8),
                        "u16" => return Ok(ForeignType::U16),
                        "u32" => return Ok(ForeignType::U32),
                        "u64" => return Ok(ForeignType::U64),
                        "f32" => return Ok(ForeignType::F32),
                        "f64" => return Ok(ForeignType::F64),
                        "String" => return Ok(ForeignType::String),
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                    let inner_type = Self::from_type(inner)?;
                                    return Ok(ForeignType::Option(Box::new(inner_type)));
                                }
                            }
                            return Err(syn::Error::new(ty.span(), "Invalid Option type"));
                        }
                        "Result" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                let mut iter = args.args.iter();
                                if let (
                                    Some(syn::GenericArgument::Type(ok_ty)),
                                    Some(syn::GenericArgument::Type(err_ty)),
                                ) = (iter.next(), iter.next())
                                {
                                    let ok_type = Self::from_type(ok_ty)?;
                                    let err_type = Self::from_type(err_ty)?;
                                    return Ok(ForeignType::Result(
                                        Box::new(ok_type),
                                        Box::new(err_type),
                                    ));
                                }
                            }
                            return Err(syn::Error::new(ty.span(), "Invalid Result type"));
                        }
                        _ => {
                            // Assume it's an opaque type reference
                            return Ok(ForeignType::OpaqueRef(segment.ident.clone()));
                        }
                    }
                }
                Err(syn::Error::new(ty.span(), "Unsupported type path"))
            }
            Type::Reference(type_ref) => {
                if let Type::Path(path) = &*type_ref.elem {
                    if path.path.is_ident("str") {
                        return Ok(ForeignType::StrRef);
                    }
                    // Check for &OpaqueType
                    if path.path.segments.len() == 1 {
                        return Ok(ForeignType::OpaqueRef(path.path.segments[0].ident.clone()));
                    }
                }
                Err(syn::Error::new(ty.span(), "Unsupported reference type"))
            }
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(ForeignType::Unit),
            _ => Err(syn::Error::new(ty.span(), "Unsupported type")),
        }
    }

    /// Get the JNI signature for this type
    fn jni_signature(&self) -> String {
        match self {
            ForeignType::Bool => "Z".into(),
            ForeignType::I8 => "B".into(),
            ForeignType::I16 => "S".into(),
            ForeignType::I32 => "I".into(),
            ForeignType::I64 => "J".into(),
            ForeignType::U8 => "B".into(), // JNI doesn't have unsigned, use signed
            ForeignType::U16 => "S".into(),
            ForeignType::U32 => "I".into(),
            ForeignType::U64 => "J".into(),
            ForeignType::F32 => "F".into(),
            ForeignType::F64 => "D".into(),
            ForeignType::String | ForeignType::StrRef => "Ljava/lang/String;".into(),
            ForeignType::Option(inner) => inner.jni_signature(),
            ForeignType::Result(ok, _) => ok.jni_signature(),
            ForeignType::OpaqueRef(name) => format!("L{};", name),
            ForeignType::Unit => "V".into(),
        }
    }

    /// Generate Rust type tokens
    fn to_rust_type(&self) -> TokenStream2 {
        match self {
            ForeignType::Bool => quote! { bool },
            ForeignType::I8 => quote! { i8 },
            ForeignType::I16 => quote! { i16 },
            ForeignType::I32 => quote! { i32 },
            ForeignType::I64 => quote! { i64 },
            ForeignType::U8 => quote! { u8 },
            ForeignType::U16 => quote! { u16 },
            ForeignType::U32 => quote! { u32 },
            ForeignType::U64 => quote! { u64 },
            ForeignType::F32 => quote! { f32 },
            ForeignType::F64 => quote! { f64 },
            ForeignType::String => quote! { String },
            ForeignType::StrRef => quote! { &str },
            ForeignType::Option(inner) => {
                let inner_ty = inner.to_rust_type();
                quote! { Option<#inner_ty> }
            }
            ForeignType::Result(ok, err) => {
                let ok_ty = ok.to_rust_type();
                let err_ty = err.to_rust_type();
                quote! { Result<#ok_ty, #err_ty> }
            }
            ForeignType::OpaqueRef(name) => quote! { #name },
            ForeignType::Unit => quote! { () },
        }
    }
}

/// A foreign function argument
#[derive(Debug, Clone)]
pub struct ForeignArg {
    pub name: Ident,
    pub ty: ForeignType,
}

/// A foreign function declaration
#[derive(Debug, Clone)]
pub struct ForeignFunctionDecl {
    pub name: Ident,
    pub receiver: Option<Ident>, // The type name if first arg is `this: &TypeName`
    pub args: Vec<ForeignArg>,
    pub return_type: ForeignType,
}

impl ForeignFunctionDecl {
    fn from_foreign_fn(func: &ForeignItemFn) -> syn::Result<Self> {
        let name = func.sig.ident.clone();
        let mut receiver = None;
        let mut args = Vec::new();

        for (i, input) in func.sig.inputs.iter().enumerate() {
            match input {
                syn::FnArg::Typed(pat_type) => {
                    let arg_name = match &*pat_type.pat {
                        Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                        _ => {
                            return Err(syn::Error::new(
                                pat_type.pat.span(),
                                "Expected identifier pattern",
                            ))
                        }
                    };

                    let arg_ty = ForeignType::from_type(&pat_type.ty)?;

                    // Check if first arg is `this: &SomeType`
                    if i == 0 && arg_name == "this" {
                        if let ForeignType::OpaqueRef(type_name) = &arg_ty {
                            receiver = Some(type_name.clone());
                            continue; // Don't add to args
                        }
                    }

                    args.push(ForeignArg {
                        name: arg_name,
                        ty: arg_ty,
                    });
                }
                syn::FnArg::Receiver(_) => {
                    return Err(syn::Error::new(
                        input.span(),
                        "Use `this: &Self` instead of `self`",
                    ));
                }
            }
        }

        let return_type = match &func.sig.output {
            ReturnType::Default => ForeignType::Unit,
            ReturnType::Type(_, ty) => ForeignType::from_type(ty)?,
        };

        Ok(Self {
            name,
            receiver,
            args,
            return_type,
        })
    }
}

/// The main parser for the `#[ffi]` attribute macro
pub struct FfiBridgeParser {
    /// Source folder path (relative to CARGO_MANIFEST_DIR)
    pub source_path: String,
    /// The foreign ABI (Swift or Kotlin)
    pub abi: ForeignAbi,
    /// Type declarations
    pub types: Vec<ForeignTypeDecl>,
    /// Function declarations
    pub functions: Vec<ForeignFunctionDecl>,
}

/// Parser for the attribute: `#[ffi("/src/ios")]`
pub struct FfiAttribute {
    pub source_path: String,
}

impl Parse for FfiAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: LitStr = input.parse()?;
        Ok(FfiAttribute {
            source_path: lit.value(),
        })
    }
}

impl FfiBridgeParser {
    /// Parse the attribute and item together
    pub fn parse_with_attr(attr: FfiAttribute, item: ItemForeignMod) -> syn::Result<Self> {
        let source_path = attr.source_path;

        // Determine the ABI
        let abi = match &item.abi.name {
            Some(name) => match name.value().as_str() {
                "Swift" => ForeignAbi::Swift,
                "Kotlin" => ForeignAbi::Kotlin,
                other => {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("Unsupported ABI: {}. Expected 'Swift' or 'Kotlin'", other),
                    ))
                }
            },
            None => {
                return Err(syn::Error::new(
                    item.abi.extern_token.span,
                    "Expected ABI string (e.g., extern \"Swift\")",
                ))
            }
        };

        let mut types = Vec::new();
        let mut functions = Vec::new();

        for item in &item.items {
            match item {
                ForeignItem::Type(ty) => {
                    types.push(ForeignTypeDecl {
                        name: ty.ident.clone(),
                    });
                }
                ForeignItem::Fn(func) => {
                    functions.push(ForeignFunctionDecl::from_foreign_fn(func)?);
                }
                _ => {
                    return Err(syn::Error::new(
                        item.span(),
                        "Only type and function declarations are supported in FFI blocks",
                    ));
                }
            }
        }

        Ok(Self {
            source_path,
            abi,
            types,
            functions,
        })
    }

    /// Generate all the code
    pub fn generate(&self) -> TokenStream2 {
        match self.abi {
            ForeignAbi::Kotlin => self.generate_android(),
            ForeignAbi::Swift => self.generate_ios(),
        }
    }

    /// Extract Android namespace from build.gradle.kts
    /// Looks for `namespace = "com.example.foo"` and converts to JNI format `com/example/foo`
    fn extract_android_namespace(&self) -> Option<String> {
        // Get the manifest dir from environment (set by cargo during compilation)
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let source_path = std::path::Path::new(&manifest_dir).join(&self.source_path);
        let build_gradle = source_path.join("build.gradle.kts");

        if !build_gradle.exists() {
            return None;
        }

        let contents = std::fs::read_to_string(&build_gradle).ok()?;

        // Look for namespace = "com.example.foo" pattern
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("namespace") {
                // Extract the quoted string
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let namespace = &trimmed[start + 1..start + 1 + end];
                        // Convert dots to slashes for JNI format
                        return Some(namespace.replace('.', "/"));
                    }
                }
            }
        }

        None
    }

    /// Generate Android JNI code
    fn generate_android(&self) -> TokenStream2 {
        let mut output = TokenStream2::new();

        // Try to extract namespace from build.gradle.kts
        let namespace = self
            .extract_android_namespace()
            .unwrap_or_else(|| "com/example".to_string());

        // Generate opaque type wrappers
        for ty in &self.types {
            let name = &ty.name;
            // Use namespace from build.gradle.kts or default
            let class_name = format!("{}/{}", namespace, name);
            let class_name_lit = syn::LitStr::new(&class_name, proc_macro2::Span::call_site());

            let type_def = quote! {
                /// Opaque wrapper around a JNI GlobalRef
                pub struct #name {
                    inner: manganis::jni::objects::GlobalRef,
                }

                impl #name {
                    /// Create a new instance by looking up the Java class at runtime
                    pub fn new() -> Result<Self, String> {
                        // Use with_activity which returns Option<R>, wrapping our Result in the Option
                        let inner_result: Option<Result<Self, String>> = manganis::android::with_activity(|mut env, activity| {
                            // Find the class
                            let class_result = env.find_class(#class_name_lit);
                            let class = match class_result {
                                Ok(c) => c,
                                Err(e) => return Some(Err(format!("Failed to find class {}: {:?}", #class_name_lit, e))),
                            };

                            // Create a new instance with Activity parameter
                            // The Kotlin plugin constructor takes (Activity) as parameter
                            let instance = match env.new_object(
                                &class,
                                "(Landroid/app/Activity;)V",
                                &[manganis::jni::objects::JValue::Object(&activity)],
                            ) {
                                Ok(i) => i,
                                Err(e) => return Some(Err(format!("Failed to create instance of {}: {:?}", #class_name_lit, e))),
                            };

                            // Convert to global ref
                            let global = match env.new_global_ref(&instance) {
                                Ok(g) => g,
                                Err(e) => return Some(Err(format!("Failed to create global ref: {:?}", e))),
                            };

                            Some(Ok(Self { inner: global }))
                        });

                        // Convert Option<Result<T, E>> to Result<T, E>
                        match inner_result {
                            Some(result) => result,
                            None => Err("Failed to get JNI environment".to_string()),
                        }
                    }

                    /// Create from an existing GlobalRef
                    pub fn from_global_ref(global: manganis::jni::objects::GlobalRef) -> Self {
                        Self { inner: global }
                    }

                    /// Get the underlying JObject
                    pub fn as_obj(&self) -> &manganis::jni::objects::JObject<'_> {
                        self.inner.as_obj()
                    }
                }
            };
            output.extend(type_def);
        }

        // Generate function implementations
        for func in &self.functions {
            let func_code = self.generate_android_function(func);
            output.extend(func_code);
        }

        // Generate linker metadata
        let metadata = self.generate_android_metadata();
        output.extend(metadata);

        // Wrap in cfg
        quote! {
            #[cfg(target_os = "android")]
            mod __ffi_android {
                #output
            }

            #[cfg(target_os = "android")]
            pub use __ffi_android::*;
        }
    }

    fn generate_android_function(&self, func: &ForeignFunctionDecl) -> TokenStream2 {
        let fn_name = &func.name;
        // Convert snake_case to camelCase for Kotlin/Java method name
        let method_name = to_camel_case(&func.name.to_string());

        // Build argument list for Rust function
        let mut rust_args = Vec::new();
        if let Some(receiver_type) = &func.receiver {
            rust_args.push(quote! { this: &#receiver_type });
        }
        for arg in &func.args {
            let name = &arg.name;
            let ty = arg.ty.to_rust_type();
            rust_args.push(quote! { #name: #ty });
        }

        // Build return type
        let return_type = func.return_type.to_rust_type();

        // Build JNI signature
        let jni_args: String = func.args.iter().map(|a| a.ty.jni_signature()).collect();
        let jni_ret = func.return_type.jni_signature();
        let jni_sig = format!("({}){}", jni_args, jni_ret);
        let jni_sig_lit = syn::LitStr::new(&jni_sig, proc_macro2::Span::call_site());

        // Build JNI call arguments - each arg needs separate binding before the call
        let mut arg_bindings = Vec::new();
        let mut jni_call_args = Vec::new();
        for (i, arg) in func.args.iter().enumerate() {
            let name = &arg.name;
            let binding_name = format_ident!("__jni_arg_{}", i);

            let (binding, arg_expr) = match &arg.ty {
                ForeignType::Bool => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Bool(if #name { 1 } else { 0 }); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::I8 | ForeignType::U8 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Byte(#name as i8); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::I16 | ForeignType::U16 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Short(#name as i16); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::I32 | ForeignType::U32 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Int(#name as i32); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::I64 | ForeignType::U64 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Long(#name as i64); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::F32 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Float(#name); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::F64 => (
                    quote! { let #binding_name = manganis::jni::objects::JValue::Double(#name); },
                    quote! { #binding_name.borrow() },
                ),
                ForeignType::String | ForeignType::StrRef => (
                    quote! {
                        let #binding_name = match env.new_string(#name) {
                            Ok(s) => s,
                            Err(e) => return Some(Err(format!("Failed to create JNI string: {:?}", e))),
                        };
                    },
                    quote! { (&&#binding_name).into() },
                ),
                _ => (
                    quote! { let #binding_name = #name.inner.as_obj(); },
                    quote! { (&#binding_name).into() },
                ),
            };
            arg_bindings.push(binding);
            jni_call_args.push(arg_expr);
        }

        // Build the call expression
        let call_target = if func.receiver.is_some() {
            quote! { this.inner.as_obj() }
        } else {
            quote! { &class }
        };

        let method_name_lit = syn::LitStr::new(&method_name, proc_macro2::Span::call_site());

        // Generate result conversion that takes env as a parameter
        // Note: call_method returns JValueGen<JObject<'_>> (owned), not a reference
        let result_conversion_fn = match &func.return_type {
            ForeignType::Unit => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, _result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<(), String> {
                    Ok(())
                }
            },
            ForeignType::Bool => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<bool, String> {
                    result.z()
                        .map(|v| v != 0)
                        .map_err(|e| format!("Failed to get boolean result: {:?}", e))
                }
            },
            ForeignType::I32 | ForeignType::U32 => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<i32, String> {
                    result.i()
                        .map_err(|e| format!("Failed to get int result: {:?}", e))
                }
            },
            ForeignType::I64 | ForeignType::U64 => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<i64, String> {
                    result.j()
                        .map_err(|e| format!("Failed to get long result: {:?}", e))
                }
            },
            ForeignType::F32 => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<f32, String> {
                    result.f()
                        .map_err(|e| format!("Failed to get float result: {:?}", e))
                }
            },
            ForeignType::F64 => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<f64, String> {
                    result.d()
                        .map_err(|e| format!("Failed to get double result: {:?}", e))
                }
            },
            ForeignType::String => quote! {
                fn convert_result<'a>(env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<String, String> {
                    let obj = result.l()
                        .map_err(|e| format!("Failed to get object result: {:?}", e))?;
                    if obj.is_null() {
                        return Ok(String::new());
                    }
                    let jstr: manganis::jni::objects::JString = obj.into();
                    let rust_str: String = env.get_string(&jstr)
                        .map_err(|e| format!("Failed to get string: {:?}", e))?
                        .into();
                    Ok(rust_str)
                }
            },
            ForeignType::Option(inner) => match inner.as_ref() {
                ForeignType::String => quote! {
                    fn convert_result<'a>(env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<Option<String>, String> {
                        let obj = result.l()
                            .map_err(|e| format!("Failed to get object result: {:?}", e))?;
                        if obj.is_null() {
                            Ok(None)
                        } else {
                            let jstr: manganis::jni::objects::JString = obj.into();
                            let rust_str: String = env.get_string(&jstr)
                                .map_err(|e| format!("Failed to get string: {:?}", e))?
                                .into();
                            Ok(Some(rust_str))
                        }
                    }
                },
                _ => quote! {
                    fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<Option<()>, String> {
                        let obj = result.l()
                            .map_err(|e| format!("Failed to get object result: {:?}", e))?;
                        if obj.is_null() {
                            Ok(None)
                        } else {
                            Ok(Some(()))
                        }
                    }
                },
            },
            _ => quote! {
                fn convert_result<'a>(_env: &mut manganis::jni::JNIEnv<'a>, _result: manganis::jni::objects::JValueGen<manganis::jni::objects::JObject<'a>>) -> Result<(), String> {
                    Ok(())
                }
            },
        };

        quote! {
            pub fn #fn_name(#(#rust_args),*) -> Result<#return_type, String> {
                // Use with_activity which returns Option<R>, wrapping our Result in the Option
                let inner_result: Option<Result<#return_type, String>> = manganis::android::with_activity(|mut env, _activity| {
                    // Define result conversion as a local function to avoid closure capture issues
                    #result_conversion_fn

                    // Perform the JNI call directly in the closure
                    #(#arg_bindings)*

                    let call_result = env.call_method(
                        #call_target,
                        #method_name_lit,
                        #jni_sig_lit,
                        &[#(#jni_call_args),*],
                    );

                    match call_result {
                        Ok(result) => Some(convert_result(&mut env, result)),
                        Err(e) => Some(Err(format!("JNI call failed: {:?}", e))),
                    }
                });

                // Convert Option<Result<T, E>> to Result<T, E>
                match inner_result {
                    Some(result) => result,
                    None => Err("Failed to get JNI environment".to_string()),
                }
            }
        }
    }

    fn generate_android_metadata(&self) -> TokenStream2 {
        // Get the first type name or use "plugin" as default
        let plugin_name = self
            .types
            .first()
            .map(|t| t.name.to_string().to_lowercase())
            .unwrap_or_else(|| "plugin".to_string());

        let source_path_lit = syn::LitStr::new(&self.source_path, proc_macro2::Span::call_site());
        let plugin_name_lit = syn::LitStr::new(&plugin_name, proc_macro2::Span::call_site());

        let mut hash = DefaultHasher::new();
        self.source_path.hash(&mut hash);
        plugin_name.hash(&mut hash);
        let plugin_hash = format!("{:016x}", hash.finish());

        let link_section = crate::linker::generate_link_section_inner(
            quote! { __METADATA },
            &plugin_hash,
            "__ASSETS__",
            quote! { manganis::android::metadata::serialize_android_metadata },
            quote! { manganis::android::macro_helpers::copy_bytes },
            quote! { manganis::android::metadata::AndroidMetadataBuffer },
        );

        quote! {
            const _: () = {
                const __METADATA: manganis::android::AndroidArtifactMetadata =
                    manganis::android::AndroidArtifactMetadata::new(
                        #plugin_name_lit,
                        concat!(env!("CARGO_MANIFEST_DIR"), "/", #source_path_lit),
                        "", // No extra dependencies by default
                    );

                #link_section
            };
        }
    }

    /// Generate iOS Objective-C code
    fn generate_ios(&self) -> TokenStream2 {
        let mut output = TokenStream2::new();

        // Generate opaque type wrappers
        for ty in &self.types {
            let name = &ty.name;
            let class_name_bytes = format!("{}\0", name);

            let type_def = quote! {
                /// Opaque wrapper around an Objective-C object pointer
                /// The actual Swift class is looked up dynamically at runtime after dx links everything
                pub struct #name {
                    inner: *mut manganis::objc2::runtime::AnyObject,
                }

                unsafe impl Send for #name {}
                unsafe impl Sync for #name {}

                impl #name {
                    /// Load the Swift framework bundle to make classes available.
                    ///
                    /// We use dlopen rather than build-time linking because Swift packages are compiled
                    /// after the Rust binary (we extract plugin metadata from the linker args).
                    ///
                    /// This is App Store compliant because:
                    /// - The framework is bundled inside the .app bundle (not downloaded)
                    /// - The framework is code-signed as part of the app
                    /// - No external code is loaded - only bundled, reviewed code
                    fn load_swift_framework() -> Result<(), &'static str> {
                        use std::sync::Once;
                        static LOAD_ONCE: Once = Once::new();
                        static mut LOAD_RESULT: Result<(), &'static str> = Ok(());

                        #[link(name = "System")]
                        extern "C" {
                            fn dlopen(filename: *const std::ffi::c_char, flags: std::ffi::c_int) -> *mut std::ffi::c_void;
                            fn dlerror() -> *const std::ffi::c_char;
                        }
                        const RTLD_NOW: std::ffi::c_int = 0x2;
                        const RTLD_GLOBAL: std::ffi::c_int = 0x8;

                        LOAD_ONCE.call_once(|| {
                            unsafe {
                                // Get the path to the executable
                                let exe_path = std::env::current_exe()
                                    .map_err(|_| "Failed to get executable path")
                                    .ok();

                                let framework_path = if let Some(exe) = exe_path {
                                    // For macOS: App.app/Contents/MacOS/binary -> App.app/Contents/Frameworks/
                                    // For iOS: App.app/binary -> App.app/Frameworks/
                                    let parent = exe.parent().unwrap_or(&exe);

                                    #[cfg(target_os = "macos")]
                                    let frameworks_dir = parent.parent().unwrap_or(parent).join("Frameworks");
                                    #[cfg(target_os = "ios")]
                                    let frameworks_dir = parent.join("Frameworks");

                                    let path = frameworks_dir.join("DioxusSwiftPlugins.framework/DioxusSwiftPlugins");
                                    if path.exists() {
                                        Some(path)
                                    } else {
                                        // Try versioned path for macOS
                                        let versioned = frameworks_dir.join("DioxusSwiftPlugins.framework/Versions/Current/DioxusSwiftPlugins");
                                        if versioned.exists() {
                                            Some(versioned)
                                        } else {
                                            None
                                        }
                                    }
                                } else {
                                    None
                                };

                                if let Some(path) = framework_path {
                                    let path_cstr = std::ffi::CString::new(path.to_string_lossy().as_bytes())
                                        .expect("Invalid framework path");

                                    // Use dlopen to load the framework
                                    let handle = dlopen(path_cstr.as_ptr(), RTLD_NOW | RTLD_GLOBAL);
                                    if handle.is_null() {
                                        let err = dlerror();
                                        if !err.is_null() {
                                            let msg = std::ffi::CStr::from_ptr(err).to_string_lossy();
                                            eprintln!("Failed to load Swift framework: {}", msg);
                                        }
                                        LOAD_RESULT = Err("Failed to load Swift framework with dlopen");
                                    }
                                } else {
                                    LOAD_RESULT = Err("Swift framework not found at expected path");
                                }
                            }
                        });

                        unsafe { LOAD_RESULT }
                    }

                    /// Create a new instance by looking up the ObjC class dynamically at runtime
                    pub fn new() -> Result<Self, &'static str> {
                        // First ensure the framework is loaded
                        Self::load_swift_framework()?;

                        unsafe {
                            // Dynamic runtime lookup - the class will be available after the framework is loaded
                            let class_name = ::std::ffi::CStr::from_bytes_with_nul(#class_name_bytes.as_bytes())
                                .expect("Invalid class name");
                            let class = manganis::objc2::runtime::AnyClass::get(class_name)
                                .ok_or("Class not found - ensure Swift sources are compiled and linked")?;

                            let instance: *mut manganis::objc2::runtime::AnyObject = manganis::objc2::msg_send![class, alloc];
                            let instance: *mut manganis::objc2::runtime::AnyObject = manganis::objc2::msg_send![instance, init];

                            if instance.is_null() {
                                return Err("Failed to initialize instance");
                            }

                            Ok(Self { inner: instance })
                        }
                    }

                    /// Create from an existing object pointer
                    pub unsafe fn from_raw(ptr: *mut manganis::objc2::runtime::AnyObject) -> Self {
                        Self { inner: ptr }
                    }
                }
            };
            output.extend(type_def);
        }

        // Generate function implementations
        for func in &self.functions {
            let func_code = self.generate_ios_function(func);
            output.extend(func_code);
        }

        // Generate linker metadata
        let metadata = self.generate_ios_metadata();
        output.extend(metadata);

        // Wrap in cfg
        quote! {
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            mod __ffi_darwin {
                #output
            }

            #[cfg(any(target_os = "ios", target_os = "macos"))]
            pub use __ffi_darwin::*;
        }
    }

    fn generate_ios_function(&self, func: &ForeignFunctionDecl) -> TokenStream2 {
        let fn_name = &func.name;

        // Build Objective-C selector
        let selector = self.rust_to_objc_selector(&func.name.to_string(), &func.args);

        // Build argument list for Rust function
        let mut rust_args = Vec::new();
        if let Some(receiver_type) = &func.receiver {
            rust_args.push(quote! { this: &#receiver_type });
        }
        for arg in &func.args {
            let name = &arg.name;
            let ty = arg.ty.to_rust_type();
            rust_args.push(quote! { #name: #ty });
        }

        // Build return type
        let return_type = func.return_type.to_rust_type();

        // Build argument conversions (variable bindings before msg_send)
        let mut arg_conversions = Vec::new();
        let mut arg_names = Vec::new();
        for (i, arg) in func.args.iter().enumerate() {
            let name = &arg.name;
            let converted_name = format_ident!("__arg_{}", i);
            let conversion = match &arg.ty {
                ForeignType::Bool => quote! {
                    let #converted_name = manganis::objc2::runtime::Bool::new(#name);
                },
                ForeignType::String | ForeignType::StrRef => {
                    quote! {
                        let __cstr = ::std::ffi::CString::new(#name.as_bytes()).unwrap();
                        let __nsstring_class = manganis::objc2::runtime::AnyClass::get(
                            ::std::ffi::CStr::from_bytes_with_nul(b"NSString\0").unwrap()
                        ).unwrap();
                        let #converted_name: *mut manganis::objc2::runtime::AnyObject = manganis::objc2::msg_send![
                            __nsstring_class,
                            stringWithUTF8String: __cstr.as_ptr()
                        ];
                    }
                }
                _ => quote! { let #converted_name = #name; },
            };
            arg_conversions.push(conversion);
            arg_names.push(converted_name);
        }

        // Build result conversion
        let result_conversion = match &func.return_type {
            ForeignType::Unit => quote! { Ok(()) },
            ForeignType::Bool => quote! {
                Ok(result.as_bool())
            },
            ForeignType::String => quote! {
                {
                    if result.is_null() {
                        Ok(String::new())
                    } else {
                        let cstr: *const ::std::os::raw::c_char = manganis::objc2::msg_send![result, UTF8String];
                        let rust_str = ::std::ffi::CStr::from_ptr(cstr)
                            .to_str()
                            .map_err(|_| "Invalid UTF-8")?;
                        Ok(rust_str.to_owned())
                    }
                }
            },
            ForeignType::Option(inner) => match inner.as_ref() {
                ForeignType::String => quote! {
                    {
                        if result.is_null() {
                            Ok(None)
                        } else {
                            let cstr: *const ::std::os::raw::c_char = manganis::objc2::msg_send![result, UTF8String];
                            let rust_str = ::std::ffi::CStr::from_ptr(cstr)
                                .to_str()
                                .map_err(|_| "Invalid UTF-8")?;
                            Ok(Some(rust_str.to_owned()))
                        }
                    }
                },
                _ => quote! {
                    if result.is_null() {
                        Ok(None)
                    } else {
                        Ok(Some(Default::default()))
                    }
                },
            },
            _ => quote! { Ok(Default::default()) },
        };

        // Build the msg_send call
        let this_expr = if func.receiver.is_some() {
            quote! { this.inner }
        } else {
            // For static methods, we'd need the class
            quote! { class }
        };

        // Build msg_send expression with proper selector syntax
        // For Swift methods with `_` external labels, the selector is just `methodName:`
        // and we call it as: msg_send![obj, methodName: arg0]
        let msg_send_call = if func.args.is_empty() {
            // No arguments - use the simple selector (no colons)
            let selector_ident = format_ident!("{}", selector);
            quote! {
                manganis::objc2::msg_send![#this_expr, #selector_ident]
            }
        } else {
            // With arguments - the selector is `methodName:` for one arg, `methodName::` for two, etc.
            // The msg_send syntax is: msg_send![obj, methodName: arg0, _: arg1, _: arg2]
            // where `_` is used for subsequent unlabeled parameters

            let method_name = to_camel_case(&func.name.to_string());
            let method_ident = format_ident!("{}", method_name);

            // Build the msg_send call tokens
            let mut tokens = quote! { #this_expr, };

            // First argument uses the method name
            if !arg_names.is_empty() {
                let first_arg = &arg_names[0];
                tokens.extend(quote! { #method_ident: #first_arg });
            }

            // Subsequent arguments use `_` as the label (for Swift's unlabeled parameters)
            for arg in arg_names.iter().skip(1) {
                let underscore = format_ident!("_");
                tokens.extend(quote! { , #underscore: #arg });
            }

            quote! {
                manganis::objc2::msg_send![#tokens]
            }
        };

        quote! {
            pub fn #fn_name(#(#rust_args),*) -> Result<#return_type, &'static str> {
                unsafe {
                    #(#arg_conversions)*
                    let result: *mut manganis::objc2::runtime::AnyObject = #msg_send_call;

                    #result_conversion
                }
            }
        }
    }

    fn generate_ios_metadata(&self) -> TokenStream2 {
        // Get the first type name or use "plugin" as default
        let plugin_name = self
            .types
            .first()
            .map(|t| t.name.to_string())
            .unwrap_or_else(|| "Plugin".to_string());

        let source_path_lit = syn::LitStr::new(&self.source_path, proc_macro2::Span::call_site());
        let plugin_name_lit =
            syn::LitStr::new(&plugin_name.to_lowercase(), proc_macro2::Span::call_site());
        let product_lit = syn::LitStr::new(&plugin_name, proc_macro2::Span::call_site());

        let mut hash = DefaultHasher::new();
        self.source_path.hash(&mut hash);
        plugin_name.hash(&mut hash);
        let plugin_hash = format!("{:016x}", hash.finish());

        let link_section = crate::linker::generate_link_section_inner(
            quote! { __METADATA },
            &plugin_hash,
            "__ASSETS__",
            quote! { manganis::darwin::metadata::serialize_swift_metadata },
            quote! { manganis::darwin::macro_helpers::copy_bytes },
            quote! { manganis::darwin::metadata::SwiftMetadataBuffer },
        );

        quote! {
            const _: () = {
                const __METADATA: manganis::darwin::SwiftPackageMetadata =
                    manganis::darwin::SwiftPackageMetadata::new(
                        #plugin_name_lit,
                        concat!(env!("CARGO_MANIFEST_DIR"), "/", #source_path_lit),
                        #product_lit,
                    );

                #link_section
            };
        }
    }

    /// Convert a Rust function name to an Objective-C selector
    ///
    /// For Swift methods that use `_` as the external parameter label (like most FFI methods),
    /// the selector is just the method name followed by colons for each parameter.
    /// e.g., `func getCurrentPositionJson(_ optionsJson: String)` -> `getCurrentPositionJson:`
    fn rust_to_objc_selector(&self, fn_name: &str, args: &[ForeignArg]) -> String {
        let mut selector = to_camel_case(fn_name);

        // For each argument, just add a colon (assuming Swift uses _ for external labels)
        for _ in args {
            selector.push(':');
        }

        selector
    }
}

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else if i == 0 {
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}
