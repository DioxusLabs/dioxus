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
