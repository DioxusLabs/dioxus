// mod hooks;
// pub use hooks::use_context;

pub mod prelude {
    use dioxus_core::prelude::Context;
    pub fn use_state<T, G>(ctx: &Context<G>, init: impl Fn() -> T) -> (T, impl Fn(T)) {
        let g = init();
        (g, |_| {})
    }
}
