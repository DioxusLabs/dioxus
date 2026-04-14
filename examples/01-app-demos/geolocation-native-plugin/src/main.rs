//! A simple Dioxus app demonstrating how to build a native plugin using manganis.
//!
//! This example shows how to use the `#[manganis::ffi]` macro to automatically generate
//! FFI bindings between Rust and native platforms (Swift/Kotlin).
//!
//! It also demonstrates how to use the widget!() macro to bundle a Widget Extension
//! for Live Activities on iOS.

use dioxus::prelude::*;

// Import the local plugin module
mod plugin;
#[cfg(target_os = "ios")]
use plugin::LiveActivityResult;
use plugin::{Geolocation, PermissionState, PermissionStatus, Position, PositionOptions};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut geolocation = use_signal(Geolocation::new);
    let mut permission_status = use_signal(|| None::<PermissionStatus>);
    let mut last_position = use_signal(|| None::<Position>);
    let mut error = use_signal(|| None::<String>);
    let mut use_high_accuracy = use_signal(|| true);
    let mut max_age_input = use_signal(|| String::from("0"));

    let on_check_permissions = {
        move |_| match geolocation.write().check_permissions() {
            Ok(status) => {
                permission_status.set(Some(status));
                error.set(None);
            }
            Err(err) => error.set(Some(err.to_string())),
        }
    };

    let on_request_permissions = move |_| {
        let mut geo = geolocation.write();
        match geo.request_permissions(None) {
            Ok(_) => match geo.check_permissions() {
                Ok(status) => {
                    permission_status.set(Some(status));
                    error.set(None);
                }
                Err(err) => error.set(Some(err.to_string())),
            },
            Err(err) => error.set(Some(err.to_string())),
        }
    };

    let on_toggle_accuracy = move |_| use_high_accuracy.toggle();

    let on_max_age_input = move |evt: FormEvent| max_age_input.set(evt.value());

    let on_fetch_position = move |_| {
        let maximum_age = max_age_input.read().trim().parse::<u32>().unwrap_or(0);

        let options = PositionOptions {
            enable_high_accuracy: use_high_accuracy(),
            timeout: 10_000,
            maximum_age,
        };

        match geolocation.write().get_current_position(Some(options)) {
            Ok(position) => {
                last_position.set(Some(position));
                error.set(None);
            }
            Err(err) => error.set(Some(err.to_string())),
        }
    };

    let accuracy_label = if use_high_accuracy() {
        "High accuracy: on"
    } else {
        "High accuracy: off"
    };

    rsx! {
        Stylesheet { href: asset!("/assets/main.css") }

        main { class: "app",
            header { class: "hero",
                div { class: "hero__copy",
                    h1 { "Geolocation plugin demo" }
                    p { "One-shot location fetching through the Dioxus geolocation plugin.
                        Measure permissions, request access, and inspect the last fix received from the device." }
                }
            }

            div { class: "cards",
                section { class: "card",
                    h2 { "Permissions" }
                    p { class: "muted",
                        "First, inspect what the OS currently allows this app to do. \
                        On Android & iOS these calls talk to the native permission dialog APIs." }
                    div { class: "button-row",
                        button { onclick: on_check_permissions, "Check permissions" }
                        button { class: "secondary", onclick: on_request_permissions, "Request permissions" }
                    }
                    match permission_status() {
                        Some(status) => rsx! {
                            div { class: "status-grid",
                                PermissionBadge { label: "Location".to_string(), state: status.location }
                                PermissionBadge { label: "Coarse location".to_string(), state: status.coarse_location }
                            }
                        },
                        None => rsx!(p { class: "muted", "Tap “Check permissions” to see the current status." }),
                    }
                }

                section { class: "card",
                    h2 { "Current position" }
                    p { class: "muted",
                        "The plugin resolves the device location once per request (no background watch). \
                        Configure the query and then fetch the coordinates." }
                    div { class: "settings",
                        button {
                            class: if use_high_accuracy() { "toggle toggle--active" } else { "toggle" },
                            onclick: on_toggle_accuracy,
                            "{accuracy_label}"
                        }
                        label { class: "field",
                            span { "Max cached age (ms)" }
                            input {
                                r#type: "number",
                                inputmode: "numeric",
                                min: "0",
                                placeholder: "0",
                                value: "{max_age_input()}",
                                oninput: on_max_age_input,
                            }
                        }
                    }
                    button { class: "primary full-width", onclick: on_fetch_position, "Get current position" }

                    match last_position() {
                        Some(position) => {
                            let snapshot = position.clone();
                            let coords = snapshot.coords.clone();
                            rsx! {
                                div { class: "position",
                                    h3 { "Latest reading" }
                                    p { class: "muted", "Timestamp: {snapshot.timestamp} ms since Unix epoch" }
                                    div { class: "position__grid",
                                        CoordinateRow { label: "Latitude".to_string(), value: format!("{:.6}", coords.latitude) }
                                        CoordinateRow { label: "Longitude".to_string(), value: format!("{:.6}", coords.longitude) }
                                        CoordinateRow { label: "Accuracy (m)".to_string(), value: format!("{:.1}", coords.accuracy) }
                                        CoordinateRow { label: "Altitude (m)".to_string(), value: format_optional(coords.altitude) }
                                        CoordinateRow { label: "Altitude accuracy (m)".to_string(), value: format_optional(coords.altitude_accuracy) }
                                        CoordinateRow { label: "Speed (m/s)".to_string(), value: format_optional(coords.speed) }
                                        CoordinateRow { label: "Heading (°)".to_string(), value: format_optional(coords.heading) }
                                    }
                                }
                            }
                        }
                        None => rsx!(p { class: "muted", "No location fetched yet." }),
                    }
                }

                // Live Activity card (iOS only)
                LiveActivityCard { geolocation, error }
            }

            if let Some(message) = error() {
                div { class: "error-banner", "Last error: {message}" }
            }
        }
    }
}

#[component]
fn PermissionBadge(label: String, state: PermissionState) -> Element {
    let (text, class) = match state {
        PermissionState::Granted => ("Granted", "badge badge--granted"),
        PermissionState::Denied => ("Denied", "badge badge--denied"),
        PermissionState::Prompt | PermissionState::PromptWithRationale => {
            ("Needs prompt", "badge badge--prompt")
        }
    };

    rsx! {
        div { class: "permission-row",
            span { class: "muted", "{label}" }
            span { class: class, "{text}" }
        }
    }
}

#[component]
fn CoordinateRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "coordinate-row",
            span { class: "muted", "{label}" }
            strong { "{value}" }
        }
    }
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|inner| format!("{inner:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(target_os = "ios")]
#[component]
fn LiveActivityCard(
    mut geolocation: Signal<Geolocation>,
    mut error: Signal<Option<String>>,
) -> Element {
    let mut live_activity = use_signal(|| None::<LiveActivityResult>);

    let on_start_live_activity = move |_| match geolocation.write().start_live_activity() {
        Ok(result) => {
            live_activity.set(Some(result));
            error.set(None);
        }
        Err(err) => error.set(Some(err.to_string())),
    };

    let on_update_live_activity = move |_| match geolocation.write().update_live_activity() {
        Ok(_) => error.set(None),
        Err(err) => error.set(Some(err.to_string())),
    };

    let on_end_live_activity = move |_| match geolocation.write().end_live_activity() {
        Ok(_) => {
            live_activity.set(None);
            error.set(None);
        }
        Err(err) => error.set(Some(err.to_string())),
    };

    rsx! {
        section { class: "card",
            h2 { "Live Activity" }
            p { class: "muted",
                "Start a Live Activity to show your current location on the lock screen. \
                Fetch your position first, then start the activity." }
            div { class: "button-row",
                button { onclick: on_start_live_activity, "Start Activity" }
                button { class: "secondary", onclick: on_update_live_activity, "Update" }
                button { class: "secondary", onclick: on_end_live_activity, "End" }
            }
            match live_activity() {
                Some(activity) => rsx! {
                    div { class: "status-grid",
                        div { class: "permission-row",
                            span { class: "muted", "Activity ID" }
                            span { "{activity.activity_id}" }
                        }
                        div { class: "permission-row",
                            span { class: "muted", "Latitude" }
                            span { "{activity.latitude:.6}" }
                        }
                        div { class: "permission-row",
                            span { class: "muted", "Longitude" }
                            span { "{activity.longitude:.6}" }
                        }
                        div { class: "permission-row",
                            span { class: "muted", "Accuracy" }
                            span { class: "badge badge--granted", "{activity.accuracy:.1}m" }
                        }
                    }
                },
                None => rsx! {
                    p { class: "muted", "No Live Activity running." }
                },
            }
        }
    }
}

#[cfg(not(target_os = "ios"))]
#[component]
fn LiveActivityCard(geolocation: Signal<Geolocation>, error: Signal<Option<String>>) -> Element {
    VNode::empty()
}
