# Examples

These examples are fully-fledged mini Dioxus apps.

You can run them with `cargo run --example EXAMPLE_NAME`. Example:

```shell
cargo run --example hello_world
```

(Most of these examples are run through webview, so you don't need the Dioxus CLI installed)

## Basic Features

[hello_world](./hello_world.rs) - Most basic example

[readme](./readme.rs) - Counter example from the Readme

[custom_assets](./custom_assets.rs) - Include images

[custom_html](./custom_html.rs) - Customize wrapper HTML

[eval](./eval.rs) - Evaluate JS expressions

### RSX

[rsx_usage](./rsx_usage.rs) - Demo of all RSX features

[xss_safety](./xss_safety.rs) - You can include text without worrying about injections by default

### Props

[borrowed](./borrowed.rs) - Borrowed props

[inlineprops](./inlineprops.rs) - Demo of `inline_props` macro

[optional_props](./optional_props.rs) - Optional props

### CSS

[all_css](./all_css.rs) - You can specify any CSS attribute

[tailwind](./tailwind/) - You can use a library for styling

## Input Handling

[all_events](./all_events.rs) - Basic event handling demo

[filedragdrop](./filedragdrop.rs) - Handle dropped files

[form](./form.rs) - Handle form submission

[inputs](./inputs.rs) - Input values

[nested_listeners](./nested_listeners.rs) - Nested handlers and bubbling

[textarea](textarea.rs) - Text area input

### State Management

[fermi](./fermi.rs) - Fermi library for state management

[pattern_reducer](./pattern_reducer.rs) - The reducer pattern with `use_state`

[rsx_compile_fail](./rsx_compile_fail.rs)

### Async

[login_form](./login_form.rs) - Login endpoint example

[suspense](./suspense.rs) - Render placeholders while data is loading

[tasks](./tasks.rs) - Continuously run future

### SVG

[svg_basic](./svg_basic.rs)

[svg](./svg.rs)

### Performance

[framework_benchmark](./framework_benchmark.rs) - Renders a huge list

> Note: The benchmark should be run in release mode:
>
>```shell
> cargo run --example framework_benchmark --release
>```

[heavy_compute](./heavy_compute.rs) - How to deal with expensive operations

## Server-side rendering

[ssr](./ssr.rs) - Rendering RSX server-side

[hydration](./hydration.rs) - Pre-rendering with hydration

## Common Patterns

[disabled](./disabled.rs) - Disable buttons conditionally

[error_handle](./error_handle.rs) - Handle errors with early return

## Routing

[flat_router](./flat_router.rs) - Basic, flat route example

[router](./router.rs) - Router example

[link](./link.rs) - Internal, external and custom links

## Platform Features

[window_event](./window_event.rs) - Window decorations, fullscreen, minimization, etc.

[window_zoom](./window_zoom.rs) – Zoom in or out

## Example Apps

[calculator](./calculator.rs) - Simple calculator

[pattern_model](./pattern_model.rs) - Simple calculator, but using a custom struct as the model

[crm](./crm.rs) - Toy multi-page customer management app

[dog_app](./dog_app.rs) - Accesses dog API

[file_explorer](./file_explorer.rs) - File browser that uses `use_ref` to interact with the model

[todomvc](./todomvc.rs) - Todo task list example

# TODO
Missing Features
- Fine-grained reactivity
- Refs - imperative handles to elements
- Function-driven children: Pass functions to make VNodes

Missing examples
- Shared state
- Root-less element groups
- Custom elements
- Component Children: Pass children into child components
- Render To string: Render a mounted virtualdom to a string
- Testing and Debugging
