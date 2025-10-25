//! Geolocation Demo
//!
//! This example demonstrates the mobile-geolocation crate with a full UI.
//! It shows how to get location on Android and iOS with automatic permission management.
//!
//! Run on Android:
//! ```bash
//! dx build --platform android --example geolocation-demo
//! dx run --device
//! ```
//!
//! Run on iOS:
//! ```bash
//! dx build --platform ios --example geolocation-demo
//! dx run --device
//! ```

use dioxus::prelude::*;

#[cfg(any(target_os = "android", target_os = "ios"))]
use dioxus_mobile_geolocation::last_known_location;

fn main() {
    launch(app);
}

#[component]
fn app() -> Element {
    let mut location = use_signal(|| None::<(f64, f64)>);
    let mut status_message = use_signal(|| "Ready to get location".to_string());
    let mut is_loading = use_signal(|| false);

    rsx! {
        style { {include_str!("./assets/mobile_geolocation.css")} }
        
        div { class: "container",
            // Header
            div { class: "header",
                h1 { "📍 Geolocation Demo" }
                p { class: "subtitle", "Cross-platform location access with Dioxus" }
            }

            // Platform indicator
            div { class: "platform-badge",
                {platform_name()}
            }

            // Status card
            div { class: "status-card",
                div { class: "status-icon",
                    if is_loading() {
                        "⏳"
                    } else if location().is_some() {
                        "✅"
                    } else {
                        "📍"
                    }
                }
                p { class: "status-text", "{status_message}" }
            }

            // Location display
            if let Some((lat, lon)) = location() {
                div { class: "location-card",
                    h2 { "Current Location" }
                    
                    div { class: "coordinate-row",
                        span { class: "label", "Latitude:" }
                        span { class: "value", "{lat:.6}°" }
                    }
                    
                    div { class: "coordinate-row",
                        span { class: "label", "Longitude:" }
                        span { class: "value", "{lon:.6}°" }
                    }
                    
                    a {
                        class: "map-link",
                        href: "https://www.google.com/maps?q={lat},{lon}",
                        target: "_blank",
                        "🗺️ View on Google Maps"
                    }
                }
            }

            // Action buttons
            div { class: "button-group",
                button {
                    class: "btn btn-primary",
                    disabled: is_loading(),
                    onclick: move |_| {
                        is_loading.set(true);
                        status_message.set("Getting location...".to_string());
                        
                        // Get location
                        #[cfg(any(target_os = "android", target_os = "ios"))]
                        {
                            match last_known_location() {
                                Some((lat, lon)) => {
                                    location.set(Some((lat, lon)));
                                    status_message.set("Location retrieved successfully!".to_string());
                                }
                                None => {
                                    status_message.set("No location available. Please check permissions.".to_string());
                                }
                            }
                        }
                        
                        #[cfg(not(any(target_os = "android", target_os = "ios")))]
                        {
                            status_message.set("Geolocation only works on Android/iOS".to_string());
                        }
                        
                        is_loading.set(false);
                    },
                    if is_loading() {
                        "⏳ Getting Location..."
                    } else {
                        "📍 Get My Location"
                    }
                }
                
                if location().is_some() {
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| {
                            location.set(None);
                            status_message.set("Location cleared".to_string());
                        },
                        "🗑️ Clear"
                    }
                }
            }

            // Info section
            div { class: "info-section",
                h3 { "ℹ️ About" }
                
                div { class: "info-item",
                    p { class: "info-title", "Permissions" }
                    p { class: "info-text",
                        "This app uses the linker-based permission system. "
                        "Permissions are automatically embedded and injected into platform manifests."
                    }
                }
                
                div { class: "info-item",
                    p { class: "info-title", "How it works" }
                    ul { class: "info-list",
                        li { "Android: Uses LocationManager via Kotlin shim" }
                        li { "iOS: Uses CoreLocation via Swift shim" }
                        li { "Permissions: Automatically managed by Dioxus CLI" }
                    }
                }
                
                div { class: "info-item",
                    p { class: "info-title", "Troubleshooting" }
                    ul { class: "info-list",
                        li { "Make sure location services are enabled" }
                        li { "Grant location permission when prompted" }
                        li { "Try using Maps app first to get initial location fix" }
                    }
                }
            }

            // Footer
            div { class: "footer",
                p { "Built with Dioxus 🦀" }
                p { class: "footer-small", "Using dioxus-mobile-geolocation" }
            }
        }
    }
}

fn platform_name() -> &'static str {
    #[cfg(target_os = "android")]
    return "🤖 Android";
    
    #[cfg(target_os = "ios")]
    return "🍎 iOS";
    
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    return "💻 Desktop (location not supported)";
}

