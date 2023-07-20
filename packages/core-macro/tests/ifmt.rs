use dioxus_core_macro::*;

#[test]
fn formatting_compiles() {
    let x = (0, 1);
    // escape sequences work
    assert_eq!(
        format_args_f!("{x:?} {{}}}}").to_string(),
        format!("{x:?} {{}}}}")
    );
    assert_eq!(
        format_args_f!("{{{{}} {x:?}").to_string(),
        format!("{{{{}} {x:?}")
    );

    // paths in formating works
    assert_eq!(format_args_f!("{x.0}").to_string(), format!("{}", x.0));

    // function calls in formatings work
    assert_eq!(
        format_args_f!("{blah(&x):?}").to_string(),
        format!("{:?}", blah(&x))
    );

    // allows duplicate format args
    assert_eq!(
        format_args_f!("{x:?} {x:?}").to_string(),
        format!("{x:?} {x:?}")
    );
}

fn blah(hi: &(i32, i32)) -> String {
    format_args_f!("{hi.0} {hi.1}").to_string()
}
