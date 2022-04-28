macro_rules! missing_trailing_comma {
    ($span:expr) => {
        proc_macro_error::emit_error!($span, "missing trailing comma")
    };
}

macro_rules! attr_after_element {
    ($span:expr) => {
        proc_macro_error::emit_error!(
            $span,
            "expected element";
            help = "move the attribute above all the children and text elements"
        )
    };
}

macro_rules! component_path_cannot_have_arguments {
    ($span:expr) => {
        proc_macro_error::abort!(
            $span,
            "expected a path without arguments";
            help = "try remove the path arguments"
        )
    };
}

macro_rules! component_ident_cannot_use_paren {
    ($span:expr, $com_name:ident) => {
        proc_macro_error::abort!(
            $span,
            "Invalid component syntax";
            help = "try replace {} (..) to {} {{..}}", $com_name, $com_name;
        )
    };
}

macro_rules! invalid_component_path {
    ($span:expr) => {
        proc_macro_error::abort!($span, "Invalid component path syntax")
    };
}
