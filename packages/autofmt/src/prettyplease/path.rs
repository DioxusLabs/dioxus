use super::algorithm::Printer;
use super::iter::IterDelimited;
use super::INDENT;
use syn::{
    AngleBracketedGenericArguments, Binding, Constraint, Expr, GenericArgument,
    ParenthesizedGenericArguments, Path, PathArguments, PathSegment, QSelf,
};

impl Printer {
    pub fn path(&mut self, path: &Path) {
        assert!(!path.segments.is_empty());
        for segment in path.segments.iter().delimited() {
            if !segment.is_first || path.leading_colon.is_some() {
                self.word("::");
            }
            self.path_segment(&segment);
        }
    }

    pub fn path_segment(&mut self, segment: &PathSegment) {
        self.ident(&segment.ident);
        self.path_arguments(&segment.arguments);
    }

    fn path_arguments(&mut self, arguments: &PathArguments) {
        match arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(arguments) => {
                self.angle_bracketed_generic_arguments(arguments);
            }
            PathArguments::Parenthesized(arguments) => {
                self.parenthesized_generic_arguments(arguments);
            }
        }
    }

    fn generic_argument(&mut self, arg: &GenericArgument) {
        match arg {
            GenericArgument::Lifetime(lifetime) => self.lifetime(lifetime),
            GenericArgument::Type(ty) => self.ty(ty),
            GenericArgument::Binding(binding) => self.binding(binding),
            GenericArgument::Constraint(constraint) => self.constraint(constraint),
            GenericArgument::Const(expr) => {
                match expr {
                    Expr::Lit(expr) => self.expr_lit(expr),
                    Expr::Block(expr) => self.expr_block(expr),
                    // ERROR CORRECTION: Add braces to make sure that the
                    // generated code is valid.
                    _ => {
                        self.word("{");
                        self.expr(expr);
                        self.word("}");
                    }
                }
            }
        }
    }

    fn angle_bracketed_generic_arguments(&mut self, generic: &AngleBracketedGenericArguments) {
        if generic.colon2_token.is_some() {
            self.word("::");
        }
        self.word("<");
        self.cbox(INDENT);
        self.zerobreak();

        // Print lifetimes before types and consts, all before bindings,
        // regardless of their order in self.args.
        //
        // TODO: ordering rules for const arguments vs type arguments have
        // not been settled yet. https://github.com/rust-lang/rust/issues/44580
        for arg in generic.args.iter().delimited() {
            match *arg {
                GenericArgument::Lifetime(_) => {
                    self.generic_argument(&arg);
                    self.trailing_comma(arg.is_last);
                }
                GenericArgument::Type(_)
                | GenericArgument::Binding(_)
                | GenericArgument::Constraint(_)
                | GenericArgument::Const(_) => {}
            }
        }
        for arg in generic.args.iter().delimited() {
            match *arg {
                GenericArgument::Type(_) | GenericArgument::Const(_) => {
                    self.generic_argument(&arg);
                    self.trailing_comma(arg.is_last);
                }
                GenericArgument::Lifetime(_)
                | GenericArgument::Binding(_)
                | GenericArgument::Constraint(_) => {}
            }
        }
        for arg in generic.args.iter().delimited() {
            match *arg {
                GenericArgument::Binding(_) | GenericArgument::Constraint(_) => {
                    self.generic_argument(&arg);
                    self.trailing_comma(arg.is_last);
                }
                GenericArgument::Lifetime(_)
                | GenericArgument::Type(_)
                | GenericArgument::Const(_) => {}
            }
        }

        self.offset(-INDENT);
        self.end();
        self.word(">");
    }

    fn binding(&mut self, binding: &Binding) {
        self.ident(&binding.ident);
        self.word(" = ");
        self.ty(&binding.ty);
    }

    fn constraint(&mut self, constraint: &Constraint) {
        self.ident(&constraint.ident);
        self.ibox(INDENT);
        for bound in constraint.bounds.iter().delimited() {
            if bound.is_first {
                self.word(": ");
            } else {
                self.space();
                self.word("+ ");
            }
            self.type_param_bound(&bound);
        }
        self.end();
    }

    fn parenthesized_generic_arguments(&mut self, arguments: &ParenthesizedGenericArguments) {
        self.cbox(INDENT);
        self.word("(");
        self.zerobreak();
        for ty in arguments.inputs.iter().delimited() {
            self.ty(&ty);
            self.trailing_comma(ty.is_last);
        }
        self.offset(-INDENT);
        self.word(")");
        self.return_type(&arguments.output);
        self.end();
    }

    pub fn qpath(&mut self, qself: &Option<QSelf>, path: &Path) {
        let qself = match qself {
            Some(qself) => qself,
            None => {
                self.path(path);
                return;
            }
        };

        assert!(qself.position < path.segments.len());

        self.word("<");
        self.ty(&qself.ty);

        let mut segments = path.segments.iter();
        if qself.position > 0 {
            self.word(" as ");
            for segment in segments.by_ref().take(qself.position).delimited() {
                if !segment.is_first || path.leading_colon.is_some() {
                    self.word("::");
                }
                self.path_segment(&segment);
                if segment.is_last {
                    self.word(">");
                }
            }
        } else {
            self.word(">");
        }
        for segment in segments {
            self.word("::");
            self.path_segment(segment);
        }
    }
}
