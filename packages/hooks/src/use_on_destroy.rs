use dioxus_core::prelude::use_drop;

#[deprecated(
    note = "Use `use_on_destroy` instead, which has the same functionality. \
This is deprecated because of the introduction of `use_on_create` which is better mirrored by `use_on_destroy`. \
The reason why `use_on_create` is not `use_on_mount` is because of potential confusion with `dioxus::events::onmounted`."
)]
pub fn use_on_unmount<D: FnOnce() + 'static>(destroy: D) {
    use_drop(destroy);
}
