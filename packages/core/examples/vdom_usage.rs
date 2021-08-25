use std::time::Duration;

use dioxus_core::prelude::*;

#[async_std::main]
async fn main() {
    static App: FC<()> = |cx| cx.render(LazyNodes::new(|f| f.text(format_args!("hello"))));

    let mut dom = VirtualDom::new(App);

    dom.rebuild();

    let mut deadline = async_std::task::sleep(Duration::from_millis(50));
    let fut = dom.run_with_deadline(deadline);

    if let Some(mutations) = fut.await {
        //
    }
}
