use dioxus::prelude::*;
use dioxus_ssr::TextRenderer;

// todo @Jon, support components in the html! macro
// let renderer = TextRenderer::new(|_| html! {<Example name="world"/>});
fn main() {
    let renderer = TextRenderer::<()>::new(|_| html! {<div> "Hello world" </div>});
    let output = renderer.render();
}

/// An example component that demonstrates how to use the functional_component macro
/// This macro makes writing functional components elegant, similar to how Rocket parses URIs.
///
/// You don't actually *need* this macro to be productive, but it makes life easier, and components cleaner.
/// This approach also integrates well with tools like Rust-Analyzer.
///
/// Notice that Context is normally generic over props, but RA doesn't care when in proc-macro mode.
/// Also notice that ctx.props still works like you would expect, so migrating to the macro is easy.
#[fc]
fn example(ctx: &Context, name: String) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}

/*
TODO

/// The macro can also be applied to statics in order to make components less verbose
/// The FC type automatically adds the inference, and property fields are automatically added as function arguments
#[fc]
static Example: FC = |ctx, name: String| {
    html! { <div> "Hello, {name}!" </div> }
};
*/

// This trait is not exposed to users directly, though they could manually implement this for struct-style components
trait Comp {
    type Props: Properties;
    fn render(&self, ctx: &mut Context<Self::Props>) -> VNode;
    fn builder(&self) -> Self::Props;
}
trait Properties {
    fn new() -> Self;
}

impl<T: Properties> Comp for FC<T> {
    type Props = T;

    fn render(&self, ctx: &mut Context<T>) -> VNode {
        let g = self(ctx);
        g
    }

    fn builder(&self) -> T {
        T::new()
    }
}

// impl<T: Properties, F: Fn(&Context<T>) -> VNode> Comp for F {
//     type Props = T;

//     fn render(&self, ctx: &mut Context<T>) -> VNode {
//         let g = self(ctx);
//         g
//     }

//     fn builder(&self) -> T {
//         T::new()
//     }
// }

impl Properties for () {
    fn new() -> Self {
        ()
    }
}
#[allow(unused, non_upper_case_globals)]
static MyComp: FC<()> = |ctx| {
    html! {
        <div>
        </div>
    }
};

fn my_comp(ctx: &Context<()>) -> VNode {
    todo!()
}

fn test() {
    let mut ctx = Context { props: &() };
    let f = MyComp.render(&mut ctx);
    let props = MyComp.builder();

    // let f = my_comp.render(&mut ctx);
    // let props = my_comp.builder();
}
