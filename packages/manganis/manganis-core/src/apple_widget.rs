use const_serialize_07 as const_serialize;
use const_serialize_08::{ConstStr, SerializeConst};

use crate::{AssetOptions, AssetOptionsBuilder};

/// Options for an Apple Widget Extension sidecar asset (iOS/macOS).
///
/// Use this to compile a Swift package as a Widget Extension and bundle it
/// with your application. Widget Extensions appear on the lock screen (iOS)
/// or in Notification Center (macOS).
///
/// # Example
///
/// ```rust,ignore
/// use manganis::{asset, Asset};
///
/// // Compile and bundle a widget extension
/// static LOCATION_WIDGET: Asset = asset!(
///     "/widgets/LocationWidget",
///     AppleWidgetOptions::new()
///         .display_name("Location Widget")
///         .bundle_id_suffix("location-widget")
/// );
///
/// // At runtime, get the path to the installed widget
/// fn get_widget_path() {
///     let widget_path = LOCATION_WIDGET.resolve();
///     // -> App.app/PlugIns/LocationWidget.appex
/// }
/// ```
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct AppleWidgetOptions {
    /// The display name shown in system UI
    display_name: ConstStr,
    /// Suffix appended to app bundle ID for the widget
    /// e.g., "location-widget" -> com.example.myapp.location-widget
    bundle_id_suffix: ConstStr,
    /// Minimum deployment target version (e.g., "16.0")
    deployment_target: ConstStr,
}

impl Default for AppleWidgetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl AppleWidgetOptions {
    /// Create a new Apple widget asset builder
    pub const fn new() -> AssetOptionsBuilder<AppleWidgetOptions> {
        AssetOptions::apple_widget()
    }

    /// Create default Apple widget options
    pub const fn default() -> Self {
        Self {
            display_name: ConstStr::new(""),
            bundle_id_suffix: ConstStr::new(""),
            deployment_target: ConstStr::new("16.0"),
        }
    }

    /// Get the display name for the widget
    pub fn display_name(&self) -> &str {
        self.display_name.as_str()
    }

    /// Get the bundle ID suffix
    pub fn bundle_id_suffix(&self) -> &str {
        self.bundle_id_suffix.as_str()
    }

    /// Get the deployment target version
    pub fn deployment_target(&self) -> &str {
        self.deployment_target.as_str()
    }
}

impl AssetOptions {
    /// Create a new Apple widget extension asset builder
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!(
    ///     "/widgets/MyWidget",
    ///     AssetOptions::apple_widget()
    ///         .display_name("My Widget")
    ///         .bundle_id_suffix("my-widget")
    /// );
    /// ```
    pub const fn apple_widget() -> AssetOptionsBuilder<AppleWidgetOptions> {
        AssetOptionsBuilder::variant(AppleWidgetOptions::default())
    }
}

impl AssetOptionsBuilder<AppleWidgetOptions> {
    /// Set the display name shown in system UI
    ///
    /// This is the human-readable name that appears when the widget is displayed.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AppleWidgetOptions};
    /// const _: Asset = asset!("/widgets/Weather", AppleWidgetOptions::new().display_name("Weather Widget"));
    /// ```
    pub const fn display_name(mut self, name: &'static str) -> Self {
        self.variant.display_name = ConstStr::new(name);
        self
    }

    /// Set the bundle identifier suffix
    ///
    /// The full widget bundle ID will be `{app_bundle_id}.{suffix}`.
    /// For example, if your app is `com.example.myapp` and the suffix is `weather`,
    /// the widget bundle ID will be `com.example.myapp.weather`.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AppleWidgetOptions};
    /// const _: Asset = asset!("/widgets/Weather", AppleWidgetOptions::new().bundle_id_suffix("weather"));
    /// ```
    pub const fn bundle_id_suffix(mut self, suffix: &'static str) -> Self {
        self.variant.bundle_id_suffix = ConstStr::new(suffix);
        self
    }

    /// Set the minimum deployment target version
    ///
    /// Widget Extensions require iOS 16.0+ / macOS 13.0+.
    /// Defaults to "16.0".
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AppleWidgetOptions};
    /// const _: Asset = asset!("/widgets/Weather", AppleWidgetOptions::new().deployment_target("17.0"));
    /// ```
    pub const fn deployment_target(mut self, version: &'static str) -> Self {
        self.variant.deployment_target = ConstStr::new(version);
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: false, // Widget extensions have fixed paths
            variant: crate::AssetVariant::AppleWidget(self.variant),
        }
    }
}
