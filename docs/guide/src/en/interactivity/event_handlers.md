# Event Handlers

Events are interesting things that happen, usually related to something the user has done. For example, some events happen when the user clicks, scrolls, moves the mouse, or types a character. To respond to these actions and make the UI interactive, we need to handle those events.

Events are associated with elements. For example, we usually don't care about all the clicks that happen within an app, only those on a particular button. To handle events that happen on an element, we must attach the desired event handler to it.

Event handlers are similar to regular attributes, but their name usually starts with `on`- and they accept closures as values. The closure will be called whenever the event is triggered, and will be passed that event.

For example, to handle clicks on an element, we can specify an `onclick` handler:

```rust
{{#include ../../../examples/event_click.rs:rsx}}
```

## The `Event` object

Event handlers receive an [`UiEvent`](https://docs.rs/dioxus-core/latest/dioxus_core/struct.UiEvent.html) object containing information about the event. Different types of events contain different types of data. For example, mouse-related events contain [`MouseData`](https://docs.rs/dioxus/latest/dioxus/events/struct.MouseData.html), which tells you things like where the mouse was clicked and what mouse buttons were used.

In the example above, this event data was logged to the terminal:

```
Clicked! Event: UiEvent { data: MouseData { coordinates: Coordinates { screen: (468.0, 109.0), client: (73.0, 25.0), element: (63.0, 15.0), page: (73.0, 25.0) }, modifiers: (empty), held_buttons: EnumSet(), trigger_button: Some(Primary) } }
Clicked! Event: UiEvent { data: MouseData { coordinates: Coordinates { screen: (468.0, 109.0), client: (73.0, 25.0), element: (63.0, 15.0), page: (73.0, 25.0) }, modifiers: (empty), held_buttons: EnumSet(), trigger_button: Some(Primary) } }
```

To learn what the different event types provide, read the [events module docs](https://docs.rs/dioxus/latest/dioxus/events/index.html).

### Stopping propagation

When you have e.g. a `button` inside a `div`, any click on the `button` is also a click on the `div`. For this reason, Dioxus propagates the click event: first, it is triggered on the target element, then on parent elements. If you want to prevent this behavior, you can call `cancel_bubble()` on the event:

```rust
{{#include ../../../examples/event_click.rs:rsx}}
```

## Prevent Default

Some events have a default behavior. For keyboard events, this might be entering the typed character. For mouse events, this might be selecting some text.

In some instances, might want to avoid this default behavior. For this, you can add the `prevent_default` attribute with the name of the handler whose default behavior you want to stop. This attribute is special: you can attach it multiple times for multiple attributes:

```rust
{{#include ../../../examples/event_prevent_default.rs:prevent_default}}
```

Any event handlers will still be called.

> Normally, in React or JavaScript, you'd call "preventDefault" on the event in the callback. Dioxus does *not* currently support this behavior. Note: this means you cannot conditionally prevent default behavior.

## Handler Props

Sometimes, you might want to make a component that accepts an event handler. A simple example would be a `FancyButton` component, which accepts an `on_click` handler:

```rust
{{#include ../../../examples/event_handler_prop.rs:component_with_handler}}
```

Then, you can use it like any other handler:

```rust
{{#include ../../../examples/event_handler_prop.rs:usage}}
```

> Note: just like any other attribute, you can name the handlers anything you want! Though they must start with `on`, for the prop to be automatically turned into an `EventHandler` at the call site.
>
> You can also put custom data in the event, rather than e.g. `MouseData`
