
# Reusing, Importing, and Exporting Components

As your application grows in size, you'll want to start breaking your UI into components and, eventually, different files. This is a great idea to encapsulate functionality of your UI and scale your team.

In this chapter we'll cover:
- Rust's modules
- Pub/Private components
- Structure for large components

## Breaking it down
Let's say our app looks something like this:

```shell
├── Cargo.toml
└── src
    └── main.rs
```

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(Scope) -> Element {}

#[derive(PartialEq, Props)]
struct PostProps{}
fn Post(Scope<PostProps>) -> Element {}

#[derive(PartialEq, Props)]
struct VoteButtonsProps {}
fn VoteButtons(Scope<VoteButtonsProps>) -> Element {}

#[derive(PartialEq, Props)]
struct TitleCardProps {}
fn TitleCard(Scope<TitleCardProps>) -> Element {}

#[derive(PartialEq, Props)]
struct MetaCardProps {}
fn MetaCard(Scope<MetaCardProps>) -> Element {}

#[derive(PartialEq, Props)]
struct ActionCardProps {}
fn ActionCard(Scope<ActionCardProps>) -> Element {}
```

That's a lot of components for one file! We've successfully refactored our app into components, but we should probably start breaking it up into a file for each component.

## Breaking into different files

Fortunately, Rust has a built-in module system that's much cleaner than what you might be used to in JavaScript. Because `VoteButtons`, `TitleCard`, `MetaCard`, and `ActionCard` all belong to the `Post` component, let's put them all in a folder together called "post". We'll make a file for each component and move the props and render function.

```rust
// src/post/action.rs

use dioxus::prelude::*;

#[derive(PartialEq, Props)]
struct ActionCardProps {}
fn ActionCard(Scope<ActionCardProps>) -> Element {}
```

We should also create a `mod.rs` file in the `post` folder so we can use it from our `main.rs`. Our `Post` component and its props will go into this file.

```rust
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
struct PostProps {}
fn Post(Scope<PostProps>) -> Element {}
```

```shell
├── Cargo.toml
└── src
    ├── main.rs
    └── post
        ├── vote.rs
        ├── title.rs
        ├── meta.rs
        ├── action.rs
        └── mod.rs
```

In our `main.rs`, we'll want to declare the `post` module so we can access our `Post` component.

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

mod post;

fn App(Scope) -> Element {
    cx.render(rsx!{
        post::Post {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 10,
            post_time: std::Instant::now(),
            url: "example".to_string(),
            title: "Title".to_string(),
            original_poster: "me".to_string()
        }
    })
}
```

If you tried to build this app right now, you'll get an error message saying that `Post is private, try changing it to public`. This is because we haven't properly exported our component! To fix this, we need to make sure both the Props and Component are declared as "public":

```rust
// src/post/mod.rs

use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct PostProps {}
pub fn Post(Scope<PostProps>) -> Element {}
```

While we're here, we also need to make sure each of our subcomponents are included as modules and exported.

Our "post/mod.rs" file will eventually look like this:

```rust
use dioxus::prelude::*;

mod vote;
mod title;
mod meta;
mod action;

#[derive(Props, PartialEq)]
pub struct PostProps {
    id: uuid::Uuid,
    score: i32,
    comment_count: u32,
    post_time: std::time::Instant,
    url: String,
    title: String,
    original_poster: String
}

pub fn Post(Scope<PostProps>) -> Element {
    cx.render(rsx!{
        div { class: "post-container"
            vote::VoteButtons {
                score: props.score,
            }
            title::TitleCard {
                title: props.title,
                url: props.url,
            }
            meta::MetaCard {
                original_poster: props.original_poster,
                post_time: props.post_time,
            }
            action::ActionCard {
                post_id: props.id
            }
        }
    })
}
```

Ultimately, including and exporting components is governed by Rust's module system. [The Rust book is a great resource to learn about these concepts in greater detail.](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)

## Final structure:

```shell
├── Cargo.toml
└── src
    ├── main.rs
    └── post
        ├── vote.rs
        ├── title.rs
        ├── meta.rs
        ├── action.rs
        └── mod.rs
```

```rust
// main.rs:
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

mod post;

fn App(Scope) -> Element {
    cx.render(rsx!{
        post::Post {
            id: Uuid::new_v4(),
            score: 10,
            comment_count: 10,
            post_time: std::Instant::now(),
            url: "example".to_string(),
            title: "Title".to_string(),
            original_poster: "me".to_string()
        }
    })
}
```


```rust
// src/post/mod.rs
use dioxus::prelude::*;

mod vote;
mod title;
mod meta;
mod action;

#[derive(Props, PartialEq)]
pub struct PostProps {
    id: uuid::Uuid,
    score: i32,
    comment_count: u32,
    post_time: std::time::Instant,
    url: String,
    title: String,
    original_poster: String
}

pub fn Post(Scope<PostProps>) -> Element {
    cx.render(rsx!{
        div { class: "post-container"
            vote::VoteButtons {
                score: props.score,
            }
            title::TitleCard {
                title: props.title,
                url: props.url,
            }
            meta::MetaCard {
                original_poster: props.original_poster,
                post_time: props.post_time,
            }
            action::ActionCard {
                post_id: props.id
            }
        }
    })
}
```

```rust
// src/post/vote.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct VoteButtonsProps {}
pub fn VoteButtons(Scope<VoteButtonsProps>) -> Element {}
```

```rust
// src/post/title.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct TitleCardProps {}
pub fn TitleCard(Scope<TitleCardProps>) -> Element {}
```

```rust
// src/post/meta.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct MetaCardProps {}
pub fn MetaCard(Scope<MetaCardProps>) -> Element {}
```

```rust
// src/post/action.rs
use dioxus::prelude::*;

#[derive(PartialEq, Props)]
pub struct ActionCardProps {}
pub fn ActionCard(Scope<ActionCardProps>) -> Element {}
```

## Moving forward

Next chapter, we'll start to add use code to hide and show Elements with conditional rendering.

For more reading on components:

- [Components in depth]()
- [Lifecycles]()
- [The Context object]()
- [Optional Prop fields]()
