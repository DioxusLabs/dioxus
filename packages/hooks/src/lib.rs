pub mod prelude {
    use dioxus_core::prelude::Context;
    pub fn use_state<T, G>(ctx: &mut Context<G>, init: impl Fn() -> T) -> (T, impl Fn(T)) {
        let g = init();
        (g, |_| {})
    }
}
