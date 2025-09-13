# Modifying the Head

Dioxus includes a series of components that render into the head of the page:

- [Title]
- [Meta]
- [document::Link](crate::Link)
- [Script]
- [Style]

Each of these components can be used to add extra information to the head of the page. For example, you can use the `Title` component to set the title of the page, or the `Meta` component to add extra metadata to the page.

## Limitations

Components that render into the head of the page do have a few key limitations:

- With the exception of the `Title` component, all components that render into the head cannot be modified after the first time they are rendered.
- Components that render into the head will not be removed even after the component is removed from the tree.
- Components that render into the head do not have a guaranteed ordering; thus, components should ideally not be order dependent, since they may not appear in the head in the order they are defined.

## Example

```rust, no_run
# use dioxus::prelude::*;
fn RedirectToDioxusHomepageWithoutJS() -> Element {
    rsx! {
        // You can use the meta component to render a meta tag into the head of the page
        // This meta tag will redirect the user to the dioxuslabs homepage in 10 seconds
        document::Meta {
            http_equiv: "refresh",
            content: "10;url=https://dioxuslabs.com",
        }
    }
}
```

## Overcoming Ordering Issues

Since Dioxus does not guarantee head element ordering, one can use es6 imports as a more predictable way to handle dependencies.

```rust, ignore
# use dioxus::prelude::*;
static HIGHLIGHT: Asset = asset!("/assets/highlight/es/highlight.min.js");
static RUST: Asset = asset!("/assets/highlight/es/languages/rust.min.js");
static STYLE: Asset = asset!("/assets/highlight/styles/atom-one-dark.css");

fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }
        document::Script {
            type: "module",
            r#"import hljs from "{HIGHLIGHT}";
            import rust from "{RUST}";
            hljs.registerLanguage('rust', rust);
            hljs.highlightAll();"#
        }
        pre {
            code {
                class: "language-rust",
                "fn main() {{\nprintln!(\"Hello, world!\");\n}}"
            }
        }
    }
}
```

## Fullstack Rendering

Head components are compatible with fullstack rendering, but only head components that are rendered in the initial render (before suspense boundaries resolve) will be rendered into the head.

If you have any important metadata that you want to render into the head, make sure to render it outside of any pending suspense boundaries.

```rust, no_run
# use dioxus::prelude::*;
# #[component]
# fn LoadData(children: Element) -> Element { unimplemented!() }
fn App() -> Element {
    rsx! {
        // This will render in SSR
        document::Title { "My Page" }
        SuspenseBoundary {
            fallback: |_| rsx! { "Loading..." },
            LoadData {
                // This will only be rendered on the client after hydration so it may not be visible to search engines
                document::Meta { name: "description", content: "My Page" }
            }
        }
    }
}
```
