use dioxus::prelude::*;
use wifiscanner::Wifi;

fn main() {
    dioxus::launch(app)
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
    Found(Vec<Wifi>),
}

fn app() -> Element {
    let mut status =
        use_resource(|| async { tokio::task::spawn_blocking(perform_scan).await.unwrap() });
    let scanning = !status.finished();

    rsx! {
        link { rel: "stylesheet", href: "https://unpkg.com/tailwindcss@^2.0/dist/tailwind.min.css" },
        div {
            div { class: "py-8 px-6",
                div { class: "container px-4 mx-auto",
                    h2 { class: "text-2xl font-bold", "Scan for WiFi Networks" }
                    button {
                        class: "inline-block w-full md:w-auto px-6 py-3 font-medium text-white bg-indigo-500 hover:bg-indigo-600 rounded transition duration-200",
                        disabled: scanning,
                        onclick: move |_| {
                            status.restart();
                        },
                        if scanning { "Scanning" } else { "Scan" }
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

                            match &*status.read() {
                                None => rsx!(""),
                                Some(Status::NoneFound) => rsx!("No networks found. Try scanning again"),
                                Some(Status::Found(wifis)) => {
                                    // Create vector of tuples of (signal_level, wifi) for sorting by signal_level
                                    let mut sorted_wifis = wifis
                                        .iter()
                                        .map(|wif: &Wifi| (wif, wif.signal_level.parse::<f32>().unwrap()))
                                        .collect::<Vec<_>>();
                                    sorted_wifis.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                                    rsx! {
                                        tbody {
                                            for (Wifi { mac: _, ssid, channel, signal_level, security }, _) in sorted_wifis.into_iter().rev() {
                                                tr { class: "text-xs bg-gray-50",
                                                    td { class: "py-5 px-6 font-medium", "{signal_level}" }
                                                    td { class: "flex py-3 font-medium", "{ssid}" }
                                                    td { span { class: "inline-block py-1 px-2 text-white bg-green-500 rounded-full", "{channel}" } }
                                                    td {  span { class: "inline-block py-1 px-2 text-purple-500 bg-purple-50 rounded-full", "{security}" } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
