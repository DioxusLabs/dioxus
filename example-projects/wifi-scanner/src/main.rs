use dioxus::prelude::*;
use dioxus_desktop::Config;
use futures::StreamExt;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::cell::Cell;
use wifiscanner::Wifi;

fn main() {
    let (sender, receiver) = unbounded();

    let other = sender.clone();

    // launch our initial background scan in a thread
    std::thread::spawn(move || {
        let _ = other.unbounded_send(Status::Scanning);
        let _ = other.unbounded_send(perform_scan());
    });

    // launch our app on the current thread - important because we spawn a window
    dioxus_desktop::launch_with_props(
        app,
        AppProps {
            sender: Cell::new(Some(sender)),
            receiver: Cell::new(Some(receiver)),
        },
        Config::default(),
    )
}

fn perform_scan() -> Status {
    if let Ok(devices) = wifiscanner::scan() {
        if devices.is_empty() {
            Status::NoneFound
        } else {
            Status::Found(devices)
        }
    } else {
        Status::NoneFound
    }
}

enum Status {
    NoneFound,
    Scanning,
    Found(Vec<Wifi>),
}

struct AppProps {
    sender: Cell<Option<UnboundedSender<Status>>>,
    receiver: Cell<Option<UnboundedReceiver<Status>>>,
}

fn app(cx: Scope<AppProps>) -> Element {
    let status = use_state(cx, || Status::NoneFound);

    let _ = use_coroutine(cx, |_: UnboundedReceiver<()>| {
        let receiver = cx.props.receiver.take();
        let status = status.to_owned();
        async move {
            if let Some(mut receiver) = receiver {
                while let Some(msg) = receiver.next().await {
                    status.set(msg);
                }
            }
        }
    });

    cx.render(rsx!(
        link { rel: "stylesheet", href: "https://unpkg.com/tailwindcss@^2.0/dist/tailwind.min.css" },
        div {
            div { class: "py-8 px-6",
                div { class: "container px-4 mx-auto",
                    h2 { class: "text-2xl font-bold", "Scan for WiFi Networks" }
                    button {
                        class: "inline-block w-full md:w-auto px-6 py-3 font-medium text-white bg-indigo-500 hover:bg-indigo-600 rounded transition duration-200",
                        onclick: move |_| {
                            let sender = cx.props.sender.take();
                            std::thread::spawn( || {
                            if let Some(sender) = sender {
                                    let _ = sender.unbounded_send(Status::Scanning);
                                    let _ = sender.unbounded_send(perform_scan());
                                }
                            });
                        },
                        match status.get() {
                            Status::Scanning => rsx!("Scanning"),
                            _ => rsx!("Scan"),
                        }
                    }
                }
            }

            section { class: "py-8",
                div { class: "container px-4 mx-auto",
                    div { class: "p-4 mb-6 bg-white shadow rounded overflow-x-auto",
                        table { class: "table-auto w-full",
                            thead {
                                tr { class: "text-xs text-gray-500 text-left",
                                    th { class: "pl-6 pb-3 font-medium", "Strength" }
                                    th { class: "pb-3 font-medium", "Network" }
                                    th { class: "pb-3 font-medium", "Channel" }
                                    th { class: "pb-3 px-2 font-medium", "Security" }
                                }
                            }

                            match status.get() {
                                Status::Scanning => rsx!(""),
                                Status::NoneFound => rsx!("No networks found. Try scanning again"),
                                Status::Found(wifis)  => {
                                    // Create vector of tuples of (signal_level, wifi) for sorting by signal_level
                                    let mut sorted_wifis = wifis
                                        .iter()
                                        .map(|wif: &Wifi| (wif, wif.signal_level.parse::<f32>().unwrap()))
                                        .collect::<Vec<_>>();
                                    sorted_wifis.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                                    rsx! {
                                        tbody {
                                            sorted_wifis.into_iter().rev().map(|(wi, _)|{
                                                let Wifi { mac: _, ssid, channel, signal_level, security } = wi;
                                                rsx!(
                                                    tr { class: "text-xs bg-gray-50",
                                                        td { class: "py-5 px-6 font-medium", "{signal_level}" }
                                                        td { class: "flex py-3 font-medium", "{ssid}" }
                                                        td { span { class: "inline-block py-1 px-2 text-white bg-green-500 rounded-full", "{channel}" } }
                                                        td {  span { class: "inline-block py-1 px-2 text-purple-500 bg-purple-50 rounded-full", "{security}" } }
                                                    }
                                                )
                                            })
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    ))
}
