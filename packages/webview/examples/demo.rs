//! An example where the dioxus vdom is running in a native thread, interacting with webview

use std::{
    borrow::BorrowMut,
    rc::Rc,
    sync::{mpsc::channel, Arc},
};

// use async_std::{channel, task::block_on};

use dioxus_core::{dodriodiff::DiffMachine, prelude::bumpalo::Bump, prelude::*, scope};
use scope::Scope;
use web_view::Handle;
static HTML_CONTENT: &'static str = include_str!("./index.html");

enum InnerEvent {
    Initiate(Handle<()>),
}

// async_std::task::spawn(async {
// #[async_std::main]
fn main() -> anyhow::Result<()> {
    let (sender, receiver) = channel::<InnerEvent>();
    // let (sender, receiver) = channel::unbounded::<InnerEvent>();

    // let task = async_std::task::spawn(async move {
    let mut view = web_view::builder()
        .title("My Project")
        .content(web_view::Content::Html(HTML_CONTENT))
        .size(320, 480)
        .resizable(true)
        .debug(true)
        .user_data(())
        .invoke_handler(|view, arg| {
            // todo: handle events here
            println!("handling invoker");
            let handle = view.handle();
            sender.send(InnerEvent::Initiate(handle));
            Ok(())
        })
        .build()
        .unwrap();

    println!("building the diff");
    let bump = Bump::new();
    let mut diff_machine = DiffMachine::new(&bump);
    let old = html! {<div> </div>}(&bump);

    // let mut scope = Scope::new(TEST, (), None);
    // scope.run::<()>();
    let new = html! {
        <div>
            <div class="flex items-center justify-center flex-col">
                <div class="flex items-center justify-center">
                    <div class="flex flex-col bg-white rounded p-4 w-full max-w-xs">
                        // Title
                        <div class="font-bold text-xl">
                            "Jon's awesome site!!11"
                        </div>

                        // Subtext / description
                        <div class="text-sm text-gray-500">
                            "He worked so hard on it :)"
                        </div>

                        <div class="flex flex-row items-center justify-center mt-6">
                            // Main number
                            <div class="font-medium text-6xl">
                                "1337"
                            </div>
                        </div>

                        // Try another
                        <div class="flex flex-row justify-between mt-6">
                            // <a href=format!("http://localhost:8080/fib/{}", other_fib_to_try) class="underline">
                                "Legit made my own React"
                            // </a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }(&bump);

    diff_machine.diff_node(&old, &new);

    let edits = diff_machine.consume();
    let ready_edits = serde_json::to_string(&edits)?;
    let ref_edits = Arc::new(ready_edits);

    loop {
        view.step().expect("should not fail");
        // if let Some(evt) = receiver.try_recv() {}
        if let Ok(event) = receiver.try_recv() {
            match event {
                InnerEvent::Initiate(handle) => {
                    // println!("awesome, things worked");
                    let ediits = ref_edits.clone();
                    // println!("{}", ediits);
                    handle
                        .dispatch(move |view| {
                            view.eval(format!("EditListReceived(`{}`);", ediits).as_str())?;

                            Ok(())
                        })
                        .expect("Dispatch failed");
                    // let g = handle.();
                }
            }
        }
        // let event = receiver.try_recv();

        // view.eval("alert('omg');")?;
        // view.step().expect("webview should not fail")?;
    }
}

// static TEST: FC<()> = |ctx, props| {
// ctx.view(html! {
//     <div>
//         <div class="flex items-center justify-center flex-col">
//             <div class="flex items-center justify-center">
//                 <div class="flex flex-col bg-white rounded p-4 w-full max-w-xs">
//                     // Title
//                     <div class="font-bold text-xl">
//                         "Jon's awesome site!!11"
//                     </div>

//                     // Subtext / description
//                     <div class="text-sm text-gray-500">
//                         "He worked so hard on it :)"
//                     </div>

//                     <div class="flex flex-row items-center justify-center mt-6">
//                         // Main number
//                         <div class="font-medium text-6xl">
//                             "1337"
//                         </div>
//                     </div>

//                     // Try another
//                     <div class="flex flex-row justify-between mt-6">
//                         // <a href=format!("http://localhost:8080/fib/{}", other_fib_to_try) class="underline">
//                             "Legit made my own React"
//                         // </a>
//                     </div>
//                 </div>
//             </div>
//         </div>
//     </div>
// })
// };
