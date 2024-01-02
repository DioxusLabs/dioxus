#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::{use_signal, use_signal_sync, Signal};
use generational_box::SyncStorage;

fn main() {
    dioxus_desktop::launch(app);
}

#[derive(Clone, Copy)]
enum ErrorComponent {
    Read,
    ReadMut,
    ReadDropped,
}

fn app(cx: Scope) -> Element {
    let error = use_signal(cx, || None);

    render! {
        match *error() {
            Some(ErrorComponent::Read) => render! { Read {} },
            Some(ErrorComponent::ReadMut) => render! { ReadMut {} },
            Some(ErrorComponent::ReadDropped) => render! { ReadDropped {} },
            None => render! {
                button {
                    onclick: move |_| error.set(Some(ErrorComponent::Read)),
                    "Read"
                }
                button {
                    onclick: move |_| error.set(Some(ErrorComponent::ReadMut)),
                    "ReadMut"
                }
                button {
                    onclick: move |_| error.set(Some(ErrorComponent::ReadDropped)),
                    "ReadDropped"
                }
            }
        }
    }
}

fn Read(cx: Scope) -> Element {
    let signal = use_signal_sync(cx, || 0);

    let _write = signal.write();
    let _read = signal.read();

    todo!()
}

fn ReadMut(cx: Scope) -> Element {
    let signal = use_signal_sync(cx, || 0);

    let _read = signal.read();
    let _write = signal.write();

    todo!()
}

fn ReadDropped(cx: Scope) -> Element {
    let signal = use_signal_sync(cx, || None);
    if cx.generation() < 4 {
        cx.needs_update();
    }
    render! {
        if let Some(value) = &*signal() {
            render!{"{value:?}"}
        } else {
            render! {
                ReadDroppedSignalChild { parent_signal: signal }
            }
        }
    }
}

#[component]
fn ReadDroppedSignalChild(
    cx: Scope,
    parent_signal: Signal<Option<Signal<i32, SyncStorage>>, SyncStorage>,
) -> Element {
    let signal = use_signal_sync(cx, || 0);
    cx.use_hook(move || {
        parent_signal.set(Some(signal));
    });
    render! {
        "{signal}"
    }
}
