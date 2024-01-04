macro_rules! missing_trailing_comma {
    ($span:expr) => {
        return Err(syn::Error::new($span, "missing trailing comma"));
    };
}

macro_rules! attr_after_element {
    ($span:expr) => {
        return Err(syn::Error::new($span, "expected element\n  = help move the attribute above all the children and text elements"));
    };
}

macro_rules! component_path_cannot_have_arguments {
    ($span:expr) => {
        return Err(Error::new(
            $span,
            "expected a path without arguments\n  = try remove the path arguments",
        ));
    };
}

macro_rules! invalid_component_path {
    ($span:expr) => {
        return Err(Error::new($span, "Invalid component path syntax"));
    };
}
