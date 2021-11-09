use std::time::Duration;

use dioxus_core::{lazynodes::LazyNodes, prelude::*};

// #[async_std::main]
fn main() {
    static App: FC<()> =
        |(cx, props)| cx.render(Some(LazyNodes::new(move |f| f.text(format_args!("hello")))));

    let mut dom = VirtualDom::new(App);

    dom.rebuild();

    // let deadline = async_std::task::sleep(Duration::from_millis(50));

    // let _fut = dom.run_with_deadline(|| deadline.);
}
