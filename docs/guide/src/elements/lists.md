# Conditional Lists and Keys

You will often want to display multiple similar components from a collection of data.

In this chapter, you will learn:

- How to use iterators in `rsx!`
- How to filter and transform data into a list of Elements
- How to create efficient lists with keys

## Rendering data from lists

If we wanted to build the Reddit app, then we need to implement a list of data that needs to be rendered: the list of posts. This list of posts is always changing, so we cannot just hardcode the lists into our app directly, like so:

```rust
// we shouldn't ship our app with posts that don't update!
rsx!(
    div {
        Post {
            title: "Post A",
            votes: 120,
        }
        Post {
            title: "Post B",
            votes: 14,
        }
        Post {
            title: "Post C",
            votes: 999,
        }
    }
)
```

Instead, we need to transform the list of data into a list of Elements.

For convenience, `rsx!` supports any type in curly braces that implements the `IntoVnodeList` trait. Conveniently, every iterator that returns something that can be rendered as an Element also implements `IntoVnodeList`.

As a simple example, let's render a list of names. First, start with our input data:

```rust
let names = ["jim", "bob", "jane", "doe"];
```

Then, we create a new iterator by calling `iter` and then [`map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map). In our `map` function, we'll render our template.

```rust
let name_list = names.iter().map(|name| rsx!(
    li { "{name}" }
));
```

We can include this list in the final Element:

```rust
rsx!(
    ul {
        name_list
    }
)
```

Rather than storing `name_list` in a temporary variable, we could also include the iterator inline:
```rust
rsx!(
    ul {
        names.iter().map(|name| rsx!(
            li { "{name}" } 
        ))
    }
)
```

The rendered HTML list is what you would expect:
```html
<ul>
    <li> jim </li>
    <li> bob </li>
    <li> jane </li>
    <li> doe </li>
</ul>
```

## Filtering Iterators

Rust's iterators are extremely powerful, especially when used for filtering tasks. When building user interfaces, you might want to display a list of items filtered by some arbitrary check.

As a very simple example, let's set up a filter where we only list names that begin with the letter "j".

Using the list from above, let's create a new iterator. Before we render the list with `map` as in the previous example, we'll [`filter`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter) the names to only allow those that start with "j".

```rust
let name_list = names
    .iter()
    .filter(|name| name.starts_with('j'))
    .map(|name| rsx!( li { "{name}" }));
```

Rust's Iterators are very versatile – check out [their documentation](https://doc.rust-lang.org/std/iter/trait.Iterator.html) for more things you can do with them!

For keen Rustaceans: notice how we don't actually call `collect` on the name list. If we `collect`ed our filtered list into new Vec, we would need to make an allocation to store these new elements, which slows down rendering. Instead, we create an entirely new _lazy_ iterator which Dioxus will consume in the `render` call. The `render` method is extraordinarily efficient, so it's best practice to let it do most of the allocations for us.

## Keeping list items in order with `key`

The examples above demonstrate the power of iterators in `rsx!` but all share the same issue: if your array items move (e.g. due to sorting), get inserted, or get deleted, Dioxus has no way of knowing what happened. This can cause Elements to be unnecessarily removed, changed and rebuilt when all that was needed was to change their position – this is inneficient.

To solve this problem, each item in the list must be **uniquely identifiable**. You can achieve this by giving it a unique, fixed "key". In Dioxus, a key is a string that identifies an item among others in the list.

```rust
rsx!( li { key: "a" } )
```

Now, if an item has already been rendered once, Dioxus can use the key to match it up later to make the correct updates – and avoid unnecessary work.

NB: the language from this section is strongly borrowed from [React's guide on keys](https://reactjs.org/docs/lists-and-keys.html).

### Where to get your key

Different sources of data provide different sources of keys:

- _Data from a database_: If your data is coming from a database, you can use the database keys/IDs, which are unique by nature.
- _Locally generated data_: If your data is generated and persisted locally (e.g. notes in a note-taking app), keep track of keys along with your data. You can use an incrementing counter or a package like `uuid` to generate keys for new items – but make sure they stay the same for the item's lifetime.

Remember: keys let Dioxus uniquely identify an item among its siblings. A well-chosen key provides more information than the position within the array. Even if the position changes due to reordering, the key lets Dioxus identify the item throughout its lifetime.

### Rules of keys

- Keys must be unique among siblings. However, it’s okay to use the same keys for Elements in different arrays.
- An item's key must not change – **don’t generate them on the fly** while rendering. Otherwise, Dioxus will be unable to keep track of which item is which, and we're back to square one.

You might be tempted to use an item's index in the array as its key. In fact, that’s what Dioxus will use if you don’t specify a key at all. This is only acceptable if you can guarantee that the list is constant – i.e., no re-ordering, additions or deletions. In all other cases, do not use the index for the key – it will lead to the performance problems described above.

Note that if you pass the key to a [custom component](./components.md) you've made, it won't receive the key as a prop. It’s only used as a hint by Dioxus itself. If your component needs an ID, you have to pass it as a separate prop:
```rust
Post { key: "{key}", id: "{key}" }
```

## Moving on

In this section, we learned:
- How to render lists of data
- How to use iterator tools to filter and transform data
- How to use keys to render lists efficiently

Moving forward, we'll learn more about attributes.
