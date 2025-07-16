rsx! {
    div {
        // Comments
        class: "asdasd",
        "hello world"
    }
    div {
        // My comment here 1
        // My comment here 2
        // My comment here 3
        // My comment here 4
        class: "asdasd",

        // Comment here
        onclick: move |_| {
            let a = 10;
            let b = 40;
            let c = 50;
        },

        // my comment

        // This here
        "hi"
    }

    // Comment head
    div { class: "asd", "Jon" }

    // Comment head
    div {
        // Collapse
        class: "asd",
        "Jon"
    }

    // comments inline
    div { // inline
        // Collapse
        class: "asd", // super inline
        class: "asd", // super inline
        "Jon" // all the inline
        // Comments at the end too
    }

    // please dont eat me 1
    div { // please dont eat me 2
        // please dont eat me 3
    }

    // please dont eat me 1
    div { // please dont eat me 2
        // please dont eat me 3
        abc: 123,
    }

    // please dont eat me 1
    div {
        // please dont eat me 3
        abc: 123,
    }

    div {
        // I am just a comment
    }

    div {
        "text"
        // I am just a comment
    }

    div {
        div {}
        // I am just a comment
    }

    div {
        {some_expr()}
        // I am just a comment
    }

    div {
        "text" // I am just a comment
    }

    div {
        div {} // I am just a comment
    }

    div {
        {some_expr()} // I am just a comment
    }

    div {
        // Please dont eat me 1
        div {
            // Please dont eat me 2
        }
        // Please dont eat me 3
    }

    div {
        "hi"
        // Please dont eat me 1
    }
    div {
        "hi" // Please dont eat me 1
        // Please dont eat me 2
    }

    // Please dont eat me 2
    Component {}

    // Please dont eat me 1
    Component {
        // Please dont eat me 2
    }

    // Please dont eat me 1
    Component {
        // Please dont eat me 2
    }

    div {
        {
            // Please dont eat me 1
            let millis = timer
                .with(|t| {
                    t.duration()
                        .saturating_sub(
                            t.started_at.map(|x| x.elapsed()).unwrap_or(Duration::ZERO),
                        )
                        .as_millis()
                });

            // Please dont eat me 2
            format!(
                "{:02}:{:02}:{:02}.{:01}",
                millis / 1000 / 3600 % 3600, // Please dont eat me 3
                millis / 1000 / 60 % 60,
                millis / 1000 % 60,

                // Please dont eat me 4
                millis / 100 % 10,
            );

            // booo //
            let b = { yay };

            // boo // boo
            let a = {
                let a = "123 // boo 123";
                // boo // boo
                asdb
            };

            format!("{b} {a}")
            // ennd
        }
    }

    div {
        // booo //
        {yay}

        // boo // boo
        {
            let a = "123 // boo 123";
            // boo // boo
            rsx! { "{a}" }
        }
    }

    div {
        input {
            r#type: "number",
            min: 0,
            max: 99,
            value: format!("{:02}", timer.read().hours),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().hours = e.value().parse().unwrap_or(0);
            },
        }

        input {
            r#type: "number",
            min: 0,
            max: 59,
            value: format!("{:02}", timer.read().minutes),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().minutes = e.value().parse().unwrap_or(0);

                // A comment inside an expression
            },
        }

        input {
            r#type: "number",
            min: 0,
            max: 59,
            value: format!("{:02}", timer.read().seconds),
            oninput: move |e| {
                // A comment inside an expression
                timer.write().seconds = e.value().parse().unwrap_or(0);
                // A comment inside an expression
            },
        }
    }

    {
        rsx! { "{a}" }
    }

    {
        rsx! { "one" }
    }

    div {}

    div {
        onpointerdown: move |evt| {
            if (ctx.disabled)() {
                return;
            }

            // Prevent default to avoid loosing focus on the range
            evt.prevent_default();
            evt.stop_propagation();

            if current_pointer_id.read().is_some()
                || evt.trigger_button() != Some(MouseButton::Primary)
            {
                return;
            }

            current_pointer_id.set(Some(evt.data().pointer_id()));
            POINTERS
                .write()
                .push(Pointer {
                    id: evt.data().pointer_id(),
                    position: evt.client_coordinates(),
                    last_position: None,
                });

            // Handle pointer interaction
            spawn(async move {
                let Some(div_element) = div_element() else {
                    return;
                };

                // Update the bounding rect of the slider in case it moved
                if let Ok(r) = div_element.get_client_rect().await {
                    rect.set(Some(r));

                    let size = if props.horizontal { r.width() } else { r.height() };

                    // Get the mouse position relative to the slider
                    let top_left = r.origin;
                    let relative_pos = evt.client_coordinates() - top_left.cast_unit();

                    let offset = if ctx.horizontal {
                        relative_pos.x
                    } else {
                        relative_pos.y
                    };
                    let new = (offset / size) * ctx.range_size() + ctx.min;
                    granular_value.set(SliderValue::Single(new));
                    let stepped = (new / ctx.step).round() * ctx.step;
                    ctx.set_value.call(SliderValue::Single(stepped));
                }

                dragging.set(true);
            });
        },
    }

    {
        rsx! { "one two three" }
    }


    // Please dont eat me 1
    //
    // Please dont eat me 2
}
