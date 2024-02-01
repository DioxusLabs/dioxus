use dioxus::prelude::*;

fn main() {
    launch(app);
}

#[derive(Clone, Copy)]
enum ErrorComponent {
    Read,
    ReadMut,
    ReadDropped,
}

fn app() -> Element {
    let mut error = use_signal(|| None as Option<ErrorComponent>);

    rsx! {
        match error() {
            Some(ErrorComponent::Read) => rsx! { Read {} },
            Some(ErrorComponent::ReadMut) => rsx! { ReadMut {} },
            Some(ErrorComponent::ReadDropped) => rsx! { ReadDropped {} },
            None => rsx! {
                button { onclick: move |_| error.set(Some(ErrorComponent::Read)), "Read" }
                button { onclick: move |_| error.set(Some(ErrorComponent::ReadMut)), "ReadMut" }
                button { onclick: move |_| error.set(Some(ErrorComponent::ReadDropped)), "ReadDropped"}
            }
        }
    }
}

#[component]
fn Read() -> Element {
    let mut signal = use_signal_sync(|| 0);

    let _write = signal.write();
    let _read = signal.read();

    unreachable!()
}

#[component]
fn ReadMut() -> Element {
    let mut signal = use_signal_sync(|| 0);

    let _read = signal.read();
    let _write = signal.write();

    unreachable!()
}

#[component]
fn ReadDropped() -> Element {
    let signal = use_signal_sync(|| None as Option<SyncSignal<i32>>);

    if generation() < 4 {
        needs_update();
    }

    rsx! {
        if let Some(value) = signal() {
            "{value:?}"
        } else {
            ReadDroppedSignalChild { parent_signal: signal }
        }
    }
}

#[component]
fn ReadDroppedSignalChild(parent_signal: SyncSignal<Option<SyncSignal<i32>>>) -> Element {
    let signal = use_signal_sync(|| 0);

    use_hook(move || parent_signal.set(Some(signal)));

    rsx! { "{signal}" }
}
