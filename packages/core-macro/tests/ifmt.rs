use std::borrow::Borrow;

use dioxus_core_macro::*;

#[test]
fn formatting_compiles() {
    let x = (0, 1);
    // escape sequences work
    assert_eq!(
        format_args_f!("{x:?} {{}}}}").to_string(),
        format!("{:?} {{}}}}", x).to_string()
    );
    assert_eq!(
        format_args_f!("{{{{}} {x:?}").to_string(),
        format!("{{{{}} {:?}", x).to_string()
    );

    // paths in formating works
    assert_eq!(
        format_args_f!("{x.0}").to_string(),
        format!("{}", x.0).to_string()
    );

    // function calls in formatings work
    assert_eq!(
        format_args_f!("{x.borrow():?}").to_string(),
        format!("{:?}", x.borrow()).to_string()
    );
}
