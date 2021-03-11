use dioxus_core::component::fc_to_builder;
use dioxus_core::prelude::*;

static BLAH: FC<()> = |ctx, _props| {
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

static SomeComponent: FC<ExampleProps> = |ctx, _props| {
    ctx.render(rsx! {
        div { }
    })
};

fn main() {}

impl Properties for ExampleProps {
    type Builder = ExamplePropsBuilder<((),)>;
    type StaticOutput = ExampleProps;
    fn builder() -> Self::Builder {
        ExampleProps::builder()
    }

    unsafe fn into_static(self) -> Self {
        self
    }
}
