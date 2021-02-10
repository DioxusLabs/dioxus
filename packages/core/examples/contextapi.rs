use std::{marker::PhantomData, ops::Deref};

use builder::{button, div};
use dioxus_core::prelude::*;

fn main() {}
struct SomeContext {
    items: Vec<String>,
}
/*
desired behavior:

free to move the context guard around
not free to move contents of context guard into closure

rules:
can deref in a function
cannot drag the refs into the closure w
*/

static Example: FC<()> = |ctx| {
    let value = use_context(&ctx, |ctx: &SomeContext| ctx.items.last().unwrap());

    // let b = *value;
    // let v2 = *value;
    let cb = move |e| {
        // let g = b.as_str();
        // let g = (v2).as_str();
        let g = (value).as_str();
        // let g = b.as_str();
    };
    // let r = *value;
    // let r2 = *r;

    ctx.view(|bump| {
        button(bump)
            .listeners([builder::on(bump, "click", cb)])
            .finish()
    })
    // ctx.view(html! {
    //     <div>
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <div>
    //             <p> "Value is: {val}" </p>
    //         </div>
    //     </div>
    // })
};

#[derive(Clone, Copy)]
struct ContextGuard<T> {
    val: PhantomData<T>,
}

impl<'a, T> Deref for ContextGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

fn use_context<'scope, 'dope, 'a, P: Properties, I, O: 'a>(
    ctx: &'scope Context<P>,
    s: fn(&'a I) -> O,
) -> &'scope ContextGuard<O> {
    // ) -> &'scope ContextGuard<O> {
    todo!()
}
