pub use dioxus_devtools_types::*;

use dioxus_core::{ScopeId, VirtualDom};
use dioxus_signals::Writable;
use warnings::Warning;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) {
    dom.runtime().on_scope(ScopeId::ROOT, || {
        let ctx = dioxus_signals::get_global_context();

        for template in &msg.templates {
            let id = &template.location;
            let value = template.template.clone();
            if let Some(mut signal) = ctx.get_signal_with_key(id) {
                dioxus_signals::warnings::signal_read_and_write_in_reactive_scope::allow(|| {
                    dioxus_signals::warnings::signal_write_in_component_body::allow(|| {
                        signal.set(Some(value));
                    });
                });
            }
        }
    });
}

/// Connect to the devserver and handle its messages
pub fn connect(mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    // Hi!
    //
    // yes, we read-raw from a tcp socket
    // don't think about it too much :)
    //
    // we just don't want to bring in tls + tokio for just hotreloading
    std::thread::spawn(move || {
        let connect = std::net::TcpStream::connect("127.0.0.1:8080");
        let Ok(mut stream) = connect else {
            return;
        };

        loop {}

        // let mut buf = [0; 1024];
        // loop {
        //     let len = stream.read(&mut buf).unwrap();
        //     if len == 0 {
        //         break;
        //     }
        //     let msg = String::from_utf8_lossy(&buf[..len]);
        //     callback(serde_json::from_str(&msg).unwrap());
        // }
    });
}
