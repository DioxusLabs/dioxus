use dioxus::prelude::*;

pub fn app() -> Element {
    rsx! {
        <div class="container">
            <h1>"Hello, world!"</h1>
            <img src="image.png" />
            <button disabled onclick={move |_| println!("clicked")}>"Click me"</button>
            <MyComponent prop="value">"children"</MyComponent>
        </div>
    }
}
