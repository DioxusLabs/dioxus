# Getting Started
Before we start utilizing Dioxus Router, we need to initialize a Dioxus web application.

#### Required Tools
If you haven't already, make sure you install the [dioxus-cli](https://dioxuslabs.com/nightly/cli/) build tool and the rust ``wasm32-unknown-unknown`` target:
```
$ cargo install dioxus-cli
    ...
$ rustup target add wasm32-unkown-unknown
    ...
```

### Creating the Project
First, create a new cargo binary project:
```
cargo new --bin dioxus-blog
```

Next, we need to add dioxus with the web and router feature to our ``Cargo.toml`` file.
```toml
[package]
name = "dioxus-blog"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.1.8", features = ["web", "router"] }
```

Now we can start coding! Create an ``index.html`` file in the root of your project:
```html
<html>
    <head>
        <title>Dioxus Blog</title>
    </head>
    <body>
        <div id="main"></div>
    </body>
</html>
```
You can add whatever you want to this file, just ensure that you have a ``div`` with the id of ``main`` in the root of your body element. This is essentially a handle to where Dioxus will render your components.

Now move to ``src/main.rs`` and replace its contents with:
```rs
use dioxus::prelude::*;

fn main() {
    // Launch Dioxus web app
    dioxus_web::launch(app);
}

// Our root component.
fn app(cx: Scope) -> Element {
    // Render "Hello, wasm!" to the screen.
    cx.render(rsx! {
        p { "Hello, wasm!"}
    })
}
```

Our project is now setup! To make sure everything is running correctly, in the root of your project run:
```
dioxus serve --platform web
```
Then head to [http://localhost:8080](http://localhost:8080) in your browser, and you should see ``Hello, wasm!`` on your screen.

#### Conclusion
We setup a new project with Dioxus and got everything running correctly. Next we'll create a small homepage and start our journey with Dioxus Router.
