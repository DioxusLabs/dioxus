<div align="center">
  <h1>Fermi ⚛</h1>
  <p>
    <strong>Atom-based global state management solution for Dioxus</strong>
  </p>
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/fermi">
    <img src="https://img.shields.io/crates/v/fermi.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/fermi">
    <img src="https://img.shields.io/crates/d/fermi.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/fermi">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/DioxusLabs/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>
</div>

---

Fermi is a global state management solution for Dioxus that's as easy as `use_state`.

Inspired by atom-based state management solutions, all state in Fermi starts as an `atom`:

```rust, ignore
static NAME: Atom<&str> = Atom(|_| "Dioxus");
```

From anywhere in our app, we can read the value of our atom:

```rust, ignore
fn NameCard() -> Element {
    let name = use_read(&NAME);
    cx.render(rsx!{ h1 { "Hello, {name}"} })
}
```

We can also set the value of our atom, also from anywhere in our app:

```rust, ignore
fn NameCard() -> Element {
    let set_name = use_set(&NAME);
    cx.render(rsx!{
        button {
            onclick: move |_| set_name("Fermi"),
            "Set name to fermi"
        }
    })
}
```

If needed, we can update the atom's value, based on itself:

```rust, ignore
static COUNT: Atom<i32> = Atom(|_| 0);

fn Counter() -> Element {
    let mut count = use_atom_state(&COUNT);

    cx.render(rsx!{
        p {
          "{count}"
        }
        button {
            onclick: move |_| count += 1,
            "Increment counter"
        }
    })
}
```

It's that simple!

## Installation

Fermi is currently under construction, so you have to use the `master` branch to get started.

```toml
[dependencies]
fermi = { git = "https://github.com/dioxuslabs/dioxus" }
```

## Running examples

The examples here use Dioxus Desktop to showcase their functionality. To run an example, use

```sh
$ cargo run --example fermi
```

## Features

Broadly our feature set required to be released includes:

- [x] Support for Atoms
- [x] Support for AtomRef (for values that aren't `Clone`)
- [ ] Support for Atom Families
- [ ] Support for memoized Selectors
- [ ] Support for memoized SelectorFamilies
- [ ] Support for UseFermiCallback for access to fermi from async
