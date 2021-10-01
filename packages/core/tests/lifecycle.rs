//! Tests for the lifecycle of components.

use anyhow::{Context, Result};
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use std::sync::{Arc, Mutex};

mod test_logging;

const IS_LOGGING_ENABLED: bool = true;
type Shared<T> = Arc<Mutex<T>>;

#[test]
fn manual_diffing() {
    test_logging::set_up_logging(IS_LOGGING_ENABLED);

    struct AppProps {
        value: Shared<&'static str>,
    }

    static App: FC<AppProps> = |cx, props| {
        let val = props.value.lock().unwrap();
        cx.render(rsx! { div { "{val}" } })
    };

    let value = Arc::new(Mutex::new("Hello"));
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            value: value.clone(),
        },
    );

    let _ = dom.rebuild();

    *value.lock().unwrap() = "goodbye";

    let edits = dom.diff();

    log::debug!("edits: {:?}", edits);
}
