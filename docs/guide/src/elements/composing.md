# Composing Components

So far, we've talked about declaring new components and setting up their properties. However, we haven't really talked about how components work together and how your app is updated.

In this section, we'll talk about:

- Sharing data between components
- How the UI is updated from input and state changes
- Forcing renders
- How renders propagate
- 


### Rendering our posts with a PostList component

Let's start by modeling this problem with a component and some properties. 

For this example, we're going to use the borrowed component syntax since we probably have a large list of posts that we don't want to clone every time we render the Post List.

```rust
#[derive(Props, PartialEq)]
struct PostListProps<'a> {
    posts: &'a [PostData]
}
```
Next, we're going to define our component:

```rust
fn App(cx: Scope<PostList>) -> Element {
    cx.render(rsx!{
        ul { class: "post-list",
            // we can drop an iterator directly into our elements
            cx.props.posts.iter().map(|post| rsx!{
                Post {
                    title: post.title,
                    age: post.age,
                    original_poster: post.original_poster
                }
            })
        }
    })
}
```
