# Architecture

## ECS

It's often ideal to represent list-y state as an SoA (struct of arrays) instead of an AoS (array of structs). In 99% of apps, normal clone-y performance is fine. If you need more performance on top of cloning on update, IM.rs will provide fast immutable data structures.

But, if you need **extreme performance** consider the ECS (SoA) model. With ECS model, we can modify fields of an entry without invalidating selectors on neighboring fields.

This approach is for that 0.1% of apps that need peak performance. Our philosophy is that these tools should be available when needed, but you shouldn't need to reach for them often. An example use case might be a graphics editor or simulation engine where thousands of entities with many fields are rendered to the screen in realtime.

Recoil will even help you:

```rust
type TodoModel = (
    String, // title
    String // subtitle
);
const TODOS: RecoilEcs<u32, TodoModel> = |builder| {
    builder.push("SomeTitle".to_string(), "SomeSubtitle".to_string());
};
const SELECT_TITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(0).select(k);
const SELECT_SUBTITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(1).select(k);
```

Or with a custom derive macro to take care of some boilerplate and maintain readability. This macro simply generates the type tuple from the model fields and then some associated constants for indexing them.

```rust
#[derive(EcsModel)]
struct TodoModel {
    title: String,
    subtitle: String
}

// derives these impl (you don't need to write this yourself, but can if you want):
mod TodoModel {
    type Layout = (String, String);
    const title: u8 = 1;
    const subtitle: u8 = 2;
}


const TODOS: RecoilEcs<u32, TodoModel::Layout> = |builder| {};
const SELECT_TITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(TodoModel::title).select(k);
const SELECT_SUBTITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(TodoModel::subtitle).select(k);
```

## Optimization

## Selectors and references.

Because new values are inserted without touching the original, we can keep old values around. As such, it makes sense to allow borrowed data since we can continue to reference old data if the data itself hasn't changed.
However, this does lead to a greater memory overhead, so occasionally we'll want to sacrifice an a component render in order to evict old values. This will be done automatically for us due to the nature of Rc.

Also, we want selectors to have internal dependencies.

## Async Atoms

- make it so "waiting" data can be provided
- someone updates via a future IE `atom.set(|| async {})`
- RecoilJS handles this with a "loadable" that has different states (ready, waiting, err just like Rust's poll method on future)
  - I think the old value is provided until the new value is ready(?)
  - Integrates with suspense to directly await the contents

## Async atoms

```rust
let (atom, set_atom, modify_atom) = (ATOM.use_read(ctx), ATOM.use_write(ctx), ATOM.use_modify(ctx));

const Title: AsyncAtom<String> = |builder| {
    builder.on_set(|api, new_val| async {

    })
    builder.on_get(|api| async {

    })
}
```

## Async selectors

```rust
struct ContentCard {
    title: String,
    content: String,
}
const ROOT_URL: Atom<&str> = |_| "localhost:8080";

// Since we don't plan on changing any content during the lifetime of the app, a selector works fine
const ContentCards: SelectorFamily<Uuid, ContentCard> = |api, key| api.on_get_async(async {
    // Whenever the root_url changes, this atom will be re-evaluated
    let root_url = api.get(&ROOT_URL);
    let data: ContentCard = fetch(format!("{}/{}", root_url, key).await?.json().await?;
    data
})

static ContentCard: FC<()> = |ctx, props| {
    let body =  async match use_recoil_value()(props.id).await {
        Ok(content) => rsx!{in ctx, p {"{content}"} }
        Err(e) => rsx!{in ctx, p {"Failed to load"}}
    };

    rsx!{
        div {
            h1{ "Non-async data here" }}
            Suspense {
                content: {body}
                fallback: { rsx!{ div {"Loading..."} } }
            }
        }
    }
};
```
