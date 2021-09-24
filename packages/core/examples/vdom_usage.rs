use std::time::Duration;

use dioxus_core::prelude::*;

#[async_std::main]
async fn main() {
    static App: FC<()> = |cx, props| cx.render(LazyNodes::new(|f| f.text(format_args!("hello"))));

    let mut dom = VirtualDom::new(App);

    dom.rebuild();

    let deadline = async_std::task::sleep(Duration::from_millis(50));

    // let _fut = dom.run_with_deadline(|| deadline.);
}
