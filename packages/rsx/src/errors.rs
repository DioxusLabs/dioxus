macro_rules! missing_trailing_comma {
    ($span:expr) => {
        return Err(Error::new($span, "missing trailing comma"));
    };
}

macro_rules! attr_after_element {
    ($span:expr) => {
        return Err(Error::new($span, "expected element\n  = help move the attribute above all the children and text elements"));
    };
}
