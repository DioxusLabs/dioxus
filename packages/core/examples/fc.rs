use dioxus_core::component::fc_to_builder;
use dioxus_core::prelude::*;
use dioxus_core_macro::fc;

use std::marker::PhantomData;

static BLAH: FC<()> = |ctx, props| {
    let g = "asd".to_string();
    ctx.render(rsx! {
        div {
            SomeComponent {
                some_field: g
            }
        }
    })
};

#[derive(PartialEq, Props)]
pub struct ExampleProps {
    some_field: String,
}

static SomeComponent: FC<ExampleProps> = |ctx, props| {
    ctx.render(rsx! {
        div { }
    })
};

fn main() {}

impl Properties for ExampleProps {
    type Builder = ExamplePropsBuilder<((),)>;
    fn builder() -> Self::Builder {
        ExampleProps::builder()
    }
}
