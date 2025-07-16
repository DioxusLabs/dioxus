rsx! {
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
}
