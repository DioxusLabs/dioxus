# Translate

`dioxus translate` can translate some source file into Dioxus code.

```
dioxus-translate 
Translate some source file into Dioxus code

USAGE:
    dioxus translate [OPTIONS] [OUTPUT]

ARGS:
    <OUTPUT>    Output file, stdout if not present

OPTIONS:
    -c, --component      Activate debug mode
    -f, --file <FILE>    Input file
```

## Translate HTML to stdout

```
dioxus transtale --file ./index.html
```

## Output in a file

```
dioxus translate --component --file ./index.html component.rsx
```

set `component` flag will wrap `dioxus rsx` code in a component function.

## Example

```html
<div>
    <h1> Hello World </h1>
    <a href="https://dioxuslabs.com/">Link</a>
</div>
```

Translate HTML to Dioxus component code.

```rust
fn component(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Hello World" },
            a {
                href: "https://dioxuslabs.com/",
                "Link"
            }
        }
    })
}
```