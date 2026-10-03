use dioxus::prelude::*;

pub fn app() -> Element {
    rsx! {
        <div class="container">
            <h1>"Hello, world!"</h1>
            <img src="image.png" />
            <button disabled onclick={move |_| println!("clicked")}>"Click me"</button>
            <my-web-component data-count="1" "custom^attr"="value" />
            <div {..attrs} />
            <MyComponent prop="value">"children"</MyComponent>
            <some::cool::Component />
            <Outlet<R>>"child"</Outlet<R>>

            // The regular syntax can be mixed in freely
            div { class: "inner", "More content" }
            for item in items {
                <span>"{item}"</span>
            }
            if cond {
                <span>"conditional"</span>
            }
            <section>
                p { class: "regular", "regular child" }
                {some_expr}
                <ul>
                    <li>"one"</li>
                    <li>"two"</li>
                </ul>
            </section>
            <div
                class="a-very-long-class-name-that-pushes-this-over-the-line-length-limit"
                id="some-long-id-attribute-value"
                onclick={move |_| println!("clicked")}
            >
                "Attributes split across lines"
            </div>
        </div>
    }
}
