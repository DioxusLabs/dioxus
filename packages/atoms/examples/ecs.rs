//! Example: ECS Architecture for very list selectors
//! --------------------------------------------------
//! Sometimes, you need *peak* performance. Cloning and Rc might simply be too much overhead for your app.
//! If you're building CPU intense apps like graphics editors, simulations, or advanced visualizations,
//! slicing up your state beyond atoms might be desirable.
//!
//! Instead of storing groups of entities in a collection of structs, the ECS Architecture instead stores
//! an array for each field in of a struct. This tends to improve performance for batch operations on
//! individual fields at the cost of complexity. Fortunately, this ECS model is built right into Recoil,
//! making it easier than ever to enable sharded datastructures in your app.
//!
//! Instead of defining a struct for our primary datastructure, we'll instead use a type tuple, and then later
//! index that tuple to get the value we care about. Unfortunately, we lose name information wrt to each
//! type in the type tuple. This can be solved with an associated module, the derive EcsMacro, or just
//! by good documentation.
//!
//! This approach is best suited for applications where individual entries in families are very large
//! and updates to neighbors are costly in terms of Clone or field comparisons for memoization.

use dioxus::prelude::*;
use dioxus_core as dioxus;
use recoil::*;

type TodoModel = (
    bool,   // checked
    String, // name
    String, // contents
);

const TODOS: EcsModel<u32, TodoModel> = |builder| {};
// const SELECT_TITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(0).select(k);
// const SELECT_SUBTITLE: SelectorBorrowed<u32, &str> = |s, k| TODOS.field(1).select(k);

static App: FC<()> = |cx| {
    use_init_recoil_root(cx, |_| {});

    // let title = use_recoil_value(cx, &C_SELECTOR);

    let title = "";
    rsx! { in cx,
        div {
            "{title}"
            // button { onclick: {next_light}, "Next light" }
        }
    }
};

fn main() {
    wasm_bindgen_futures::spawn_local(dioxus_web::WebsysRenderer::start(App))
}
