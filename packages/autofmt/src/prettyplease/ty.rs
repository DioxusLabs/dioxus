use super::algorithm::Printer;
use super::iter::IterDelimited;
use super::INDENT;
use proc_macro2::TokenStream;
use syn::{
    Abi, BareFnArg, ReturnType, Type, TypeArray, TypeBareFn, TypeGroup, TypeImplTrait, TypeInfer,
    TypeMacro, TypeNever, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice, TypeTraitObject,
    TypeTuple, Variadic,
};

impl Printer {
    pub fn ty(&mut self, ty: &Type) {
        match ty {
            Type::Array(ty) => self.type_array(ty),
            Type::BareFn(ty) => self.type_bare_fn(ty),
            Type::Group(ty) => self.type_group(ty),
            Type::ImplTrait(ty) => self.type_impl_trait(ty),
            Type::Infer(ty) => self.type_infer(ty),
            Type::Macro(ty) => self.type_macro(ty),
            Type::Never(ty) => self.type_never(ty),
            Type::Paren(ty) => self.type_paren(ty),
            Type::Path(ty) => self.type_path(ty),
            Type::Ptr(ty) => self.type_ptr(ty),
            Type::Reference(ty) => self.type_reference(ty),
            Type::Slice(ty) => self.type_slice(ty),
            Type::TraitObject(ty) => self.type_trait_object(ty),
            Type::Tuple(ty) => self.type_tuple(ty),
            Type::Verbatim(ty) => self.type_verbatim(ty),
            #[cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
            _ => unimplemented!("unknown Type"),
        }
    }

    fn type_array(&mut self, ty: &TypeArray) {
        self.word("[");
        self.ty(&ty.elem);
        self.word("; ");
        self.expr(&ty.len);
        self.word("]");
    }

    fn type_bare_fn(&mut self, ty: &TypeBareFn) {
        if let Some(bound_lifetimes) = &ty.lifetimes {
            self.bound_lifetimes(bound_lifetimes);
        }
        if ty.unsafety.is_some() {
            self.word("unsafe ");
        }
        if let Some(abi) = &ty.abi {
            self.abi(abi);
        }
        self.word("fn(");
        self.cbox(INDENT);
        self.zerobreak();
        for bare_fn_arg in ty.inputs.iter().delimited() {
            self.bare_fn_arg(&bare_fn_arg);
            self.trailing_comma(bare_fn_arg.is_last && ty.variadic.is_none());
        }
        if let Some(variadic) = &ty.variadic {
            self.variadic(variadic);
            self.zerobreak();
        }
        self.offset(-INDENT);
        self.end();
        self.word(")");
        self.return_type(&ty.output);
    }

    fn type_group(&mut self, ty: &TypeGroup) {
        self.ty(&ty.elem);
    }

    fn type_impl_trait(&mut self, ty: &TypeImplTrait) {
        self.word("impl ");
        for type_param_bound in ty.bounds.iter().delimited() {
            if !type_param_bound.is_first {
                self.word(" + ");
            }
            self.type_param_bound(&type_param_bound);
        }
    }

    fn type_infer(&mut self, ty: &TypeInfer) {
        let _ = ty;
        self.word("_");
    }

    fn type_macro(&mut self, ty: &TypeMacro) {
        self.mac(&ty.mac, None);
    }

    fn type_never(&mut self, ty: &TypeNever) {
        let _ = ty;
        self.word("!");
    }

    fn type_paren(&mut self, ty: &TypeParen) {
        self.word("(");
        self.ty(&ty.elem);
        self.word(")");
    }

    fn type_path(&mut self, ty: &TypePath) {
        self.qpath(&ty.qself, &ty.path);
    }

    fn type_ptr(&mut self, ty: &TypePtr) {
        self.word("*");
        if ty.mutability.is_some() {
            self.word("mut ");
        } else {
            self.word("const ");
        }
        self.ty(&ty.elem);
    }

    fn type_reference(&mut self, ty: &TypeReference) {
        self.word("&");
        if let Some(lifetime) = &ty.lifetime {
            self.lifetime(lifetime);
            self.nbsp();
        }
        if ty.mutability.is_some() {
            self.word("mut ");
        }
        self.ty(&ty.elem);
    }

    fn type_slice(&mut self, ty: &TypeSlice) {
        self.word("[");
        self.ty(&ty.elem);
        self.word("]");
    }

    fn type_trait_object(&mut self, ty: &TypeTraitObject) {
        self.word("dyn ");
        for type_param_bound in ty.bounds.iter().delimited() {
            if !type_param_bound.is_first {
                self.word(" + ");
            }
            self.type_param_bound(&type_param_bound);
        }
    }

    fn type_tuple(&mut self, ty: &TypeTuple) {
        self.word("(");
        self.cbox(INDENT);
        self.zerobreak();
        for elem in ty.elems.iter().delimited() {
            self.ty(&elem);
            if ty.elems.len() == 1 {
                self.word(",");
                self.zerobreak();
            } else {
                self.trailing_comma(elem.is_last);
            }
        }
        self.offset(-INDENT);
        self.end();
        self.word(")");
    }

    fn type_verbatim(&mut self, ty: &TokenStream) {
        if ty.to_string() == "..." {
            self.word("...");
        } else {
            unimplemented!("Type::Verbatim `{}`", ty);
        }
    }

    pub fn return_type(&mut self, ty: &ReturnType) {
        match ty {
            ReturnType::Default => {}
            ReturnType::Type(_arrow, ty) => {
                self.word(" -> ");
                self.ty(ty);
            }
        }
    }

    fn bare_fn_arg(&mut self, bare_fn_arg: &BareFnArg) {
        self.outer_attrs(&bare_fn_arg.attrs);
        if let Some((name, _colon)) = &bare_fn_arg.name {
            self.ident(name);
            self.word(": ");
        }
        self.ty(&bare_fn_arg.ty);
    }

    fn variadic(&mut self, variadic: &Variadic) {
        self.outer_attrs(&variadic.attrs);
        self.word("...");
    }

    pub fn abi(&mut self, abi: &Abi) {
        self.word("extern ");
        if let Some(name) = &abi.name {
            self.lit_str(name);
            self.nbsp();
        }
    }
}
