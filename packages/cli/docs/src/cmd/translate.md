# Translate

`dioxus translate` can translate some `html` file into a Dioxus compoent

```
dioxus-translate 
Translate some source file into a Dioxus component

USAGE:
    dioxus translate [OPTIONS] [OUTPUT]

ARGS:
    <OUTPUT>    Output file, defaults to stdout if not present

OPTIONS:
    -c, --component      Activate debug mode
    -f, --file <FILE>    Input file
```

## Translate HTML to stdout

You can use the `file` option to set path to the `html` file to translate:

```
dioxus transtale --file ./index.html
```

## Output rsx to a file

You can pass a file to the traslate command to set the path to write the output of the command to:

```
dioxus translate --file ./index.html component.rsx
```

## Output rsx to a file

Setting the `component` option will create a compoent from the HTML:

```
dioxus translate --file ./index.html --component
```

## Example

This HTML:
```html
<div>
    <h1> Hello World </h1>
    <a href="https://dioxuslabs.com/">Link</a>
</div>
```

Translates into this Dioxus component:

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