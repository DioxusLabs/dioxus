//! Exprs that are too long to fit on one line

fn it_works() {
    rsx! {
        div {
            if thing
                .some_long_method_that_is_too_long_to_fit_on_one_line()
                .some_long_method_that_is_too_long_to_fit_on_one_line()
                .some_long_method_that_is_too_long_to_fit_on_one_line({
                    chain()
                        .some_long_method_that_is_too_long_to_fit_on_one_line()
                        .some_long_method_that_is_too_long_to_fit_on_one_line()
                })
            {
                "hi"
            }

            for item in thing
                .some_long_method_that_is_too_long_to_fit_on_one_line()
                .some_long_method_that_is_too_long_to_fit_on_one_line()
                .some_long_method_that_is_too_long_to_fit_on_one_line({
                    chain()
                        .some_long_method_that_is_too_long_to_fit_on_one_line()
                        .some_long_method_that_is_too_long_to_fit_on_one_line()
                })
            {
                "hi"
            }
        }
    }
}
