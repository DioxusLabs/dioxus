//! Tests for the lifecycle of components.

use std::{cell::RefCell, rc::Rc};

use anyhow::{Context, Result};
use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_html as dioxus_elements;
mod test_logging;

const IS_LOGGING_ENABLED: bool = true;
type Shared<T> = Rc<RefCell<T>>;

#[test]
fn manual_diffing() {
    test_logging::set_up_logging(IS_LOGGING_ENABLED);

    #[derive(PartialEq, Props)]
    struct AppProps {
        value: Shared<&'static str>,
    }

    static App: FC<AppProps> = |cx| {
        let val = cx.value.borrow();
        cx.render(rsx! { div { "{val}" } })
    };

    let value = Rc::new(RefCell::new("Hello"));
    let mut dom = VirtualDom::new_with_props(
        App,
        AppProps {
            value: value.clone(),
        },
    );

    let _ = dom.rebuild();

    *value.borrow_mut() = "goodbye";

    let edits = dom.diff();

    log::debug!("edits: {:?}", edits);
}
