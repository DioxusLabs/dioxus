use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut status = use_resource(|| async {
        use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};

        let manager = btleplug::platform::Manager::new().await.unwrap();

        // get the first bluetooth adapter
        let adapters = manager.adapters().await.unwrap();
        let central = adapters.into_iter().next().unwrap();

        // start scanning for devices
        central.start_scan(ScanFilter::default()).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Return the list of peripherals after scanning
        let mut devices = vec![];
        for p in central.peripherals().await.unwrap() {
            if let Some(p) = p.properties().await.unwrap() {
                devices.push(p);
            }
        }

        println!("Found {} Bluetooth devices", devices.len());

        devices
    });

    let scanning = !status.finished();

    rsx! {
        link { rel: "stylesheet", href: asset!("/assets/tailwind.css") },
        div {
            div { class: "py-8 px-6",
                div { class: "container px-4 mx-auto",
                    h2 { class: "text-2xl font-bold", "Scan for Bluetooth Devices" }
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
                                None => rsx!("no devices yet!"),
                                Some(peripherals) => {
                                    // Create vector of tuples of (signal_level, wifi) for sorting by signal_level
                                    let mut sorted_devices = peripherals.clone();
                                    sorted_devices.sort_by(|a, b| a.rssi.partial_cmp(&b.rssi).unwrap());

                                    rsx! {
                                        tbody {
                                            for peripheral in sorted_devices.into_iter().rev() {
                                                tr { class: "text-xs bg-gray-50",
                                                    td { class: "py-5 px-6 font-medium", "{peripheral.rssi.unwrap_or(-100)}" }
                                                    td { class: "flex py-3 font-medium", "{peripheral.local_name.clone().unwrap_or_default()}" }
                                                    td { span { class: "inline-block py-1 px-2 text-white bg-green-500 rounded-full", "{peripheral.address}" } }
                                                    td {  span { class: "inline-block py-1 px-2 text-purple-500 bg-purple-50 rounded-full", "{peripheral.tx_power_level.unwrap_or_default()}" } }
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
