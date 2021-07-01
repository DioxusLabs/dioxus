//! A Showcase of all the useful examples
//!
//!
//!

fn main() {
    use_css_consumer(&cx, "mystyle");

    // at the global head of the app
    use_css_provider(&cx, |cfg| {});
    use_recoil_provider(&cx, |cfg| {});

    let recoil = use_recoil_api(&cx, |_| {});
    use_websocket_connection(&cx, move |cfg| {
        cfg.on_receive(move |event| match event.data::<Msg>() {
            Ok(msg) => match msg {
                a => recoil.set(&ATOM, 10),
                c => recoil.set(&ATOM, 20),
                _ => {}
            },
            Err(e) => {}
        });
        cfg.on_close(move |event| {});
        cfg.on_open(move |event| {});
    });
}
