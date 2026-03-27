//! Maps unified Dioxus.toml config to platform-specific manifest data.
//!
//! This module converts cross-platform declarations (permissions, deep links,
//! background modes) into platform-specific identifiers:
//! - Android: `<uses-permission>` entries, intent filters, foreground service types
//! - iOS/macOS: Info.plist keys, URL schemes, UIBackgroundModes

use crate::config::{
    AndroidConfig, BackgroundConfig, DeepLinkConfig, IosConfig, LocationPrecision, MacosConfig,
    PermissionsConfig, StorageAccess,
};

/// Android permission entry for AndroidManifest.xml
#[derive(Debug, Clone)]
pub struct AndroidPermissionEntry {
    /// Full Android permission string (e.g., "android.permission.CAMERA")
    pub permission: String,
    /// User-facing description (used for documentation)
    pub description: String,
}

/// iOS/macOS plist entry for Info.plist
#[derive(Debug, Clone)]
pub struct PlistEntry {
    /// Plist key (e.g., "NSCameraUsageDescription")
    pub key: String,
    /// User-facing description shown in permission dialogs
    pub value: String,
}

/// Maps unified permissions, deep links, and background modes to platform-specific identifiers
#[derive(Debug, Default)]
pub struct ManifestMapper {
    pub android_permissions: Vec<AndroidPermissionEntry>,
    pub android_features: Vec<String>,
    pub ios_plist_entries: Vec<PlistEntry>,
    pub macos_plist_entries: Vec<PlistEntry>,

    /// URL schemes for iOS CFBundleURLTypes (merged from deep_links.schemes + ios.url_schemes)
    pub ios_url_schemes: Vec<String>,
    /// URL schemes for macOS CFBundleURLTypes (merged from deep_links.schemes + macos.url_schemes)
    pub macos_url_schemes: Vec<String>,
    /// URL schemes for Android intent-filter (merged from deep_links.schemes + android.url_schemes)
    pub android_url_schemes: Vec<String>,
    /// Associated domains for iOS (from deep_links.hosts → "applinks:host")
    pub ios_associated_domains: Vec<String>,
    /// Android intent filters from config (android.intent_filters)
    pub android_intent_filters: Vec<crate::config::AndroidIntentFilter>,
    /// App link hosts for Android auto-verify (from deep_links.hosts)
    pub android_app_link_hosts: Vec<String>,

    /// iOS UIBackgroundModes (merged from BackgroundConfig + ios.background_modes)
    pub ios_background_modes: Vec<String>,
    /// Android foreground service types (from BackgroundConfig + android.foreground_service_types)
    pub android_foreground_service_types: Vec<String>,
}

impl ManifestMapper {
    /// Create a new permission mapper from the unified config
    pub fn from_config(
        permissions: &PermissionsConfig,
        deep_links: &DeepLinkConfig,
        background: &BackgroundConfig,
        android: &AndroidConfig,
        ios: &IosConfig,
        macos: &MacosConfig,
    ) -> Self {
        let mut mapper = Self::default();

        // Map unified permissions
        mapper.map_location(permissions);
        mapper.map_camera(permissions);
        mapper.map_microphone(permissions);
        mapper.map_notifications(permissions);
        mapper.map_photos(permissions);
        mapper.map_bluetooth(permissions);
        mapper.map_background_location(permissions);
        mapper.map_contacts(permissions);
        mapper.map_calendar(permissions);
        mapper.map_biometrics(permissions);
        mapper.map_nfc(permissions);
        mapper.map_motion(permissions);
        mapper.map_health(permissions);
        mapper.map_speech(permissions);
        mapper.map_media_library(permissions);
        mapper.map_siri(permissions);
        mapper.map_homekit(permissions);
        mapper.map_local_network(permissions);
        mapper.map_nearby_wifi(permissions);

        // Add raw Android permissions
        for (perm, config) in &android.permissions {
            mapper.android_permissions.push(AndroidPermissionEntry {
                permission: perm.clone(),
                description: config.description.clone(),
            });
        }

        // Add Android features
        mapper.android_features.extend(android.features.clone());

        // Map deep links
        mapper.map_deep_links(deep_links, android, ios, macos);

        // Map background modes
        mapper.map_background_modes(background, android, ios);

        // Log mapped permissions for debugging
        for perm in &mapper.android_permissions {
            tracing::debug!(
                "Android permission: {} - {}",
                perm.permission,
                perm.description
            );
        }
        for entry in &mapper.ios_plist_entries {
            tracing::debug!("iOS plist: {} = {}", entry.key, entry.value);
        }

        mapper
    }

    fn map_location(&mut self, permissions: &PermissionsConfig) {
        if let Some(loc) = &permissions.location {
            let android_perm = match loc.precision {
                LocationPrecision::Fine => "android.permission.ACCESS_FINE_LOCATION",
                LocationPrecision::Coarse => "android.permission.ACCESS_COARSE_LOCATION",
            };

            self.android_permissions.push(AndroidPermissionEntry {
                permission: android_perm.to_string(),
                description: loc.description.clone(),
            });

            // For fine location, also add coarse as it's often needed
            if loc.precision == LocationPrecision::Fine {
                self.android_permissions.push(AndroidPermissionEntry {
                    permission: "android.permission.ACCESS_COARSE_LOCATION".to_string(),
                    description: loc.description.clone(),
                });
            }

            self.ios_plist_entries.push(PlistEntry {
                key: "NSLocationWhenInUseUsageDescription".to_string(),
                value: loc.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSLocationUsageDescription".to_string(),
                value: loc.description.clone(),
            });
        }
    }

    fn map_camera(&mut self, permissions: &PermissionsConfig) {
        if let Some(cam) = &permissions.camera {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.CAMERA".to_string(),
                description: cam.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSCameraUsageDescription".to_string(),
                value: cam.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSCameraUsageDescription".to_string(),
                value: cam.description.clone(),
            });
        }
    }

    fn map_microphone(&mut self, permissions: &PermissionsConfig) {
        if let Some(mic) = &permissions.microphone {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.RECORD_AUDIO".to_string(),
                description: mic.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSMicrophoneUsageDescription".to_string(),
                value: mic.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSMicrophoneUsageDescription".to_string(),
                value: mic.description.clone(),
            });
        }
    }

    fn map_notifications(&mut self, permissions: &PermissionsConfig) {
        if let Some(notif) = &permissions.notifications {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.POST_NOTIFICATIONS".to_string(),
                description: notif.description.clone(),
            });
            // iOS notifications are handled at runtime, no plist entry needed
        }
    }

    fn map_photos(&mut self, permissions: &PermissionsConfig) {
        if let Some(photos) = &permissions.photos {
            match photos.access {
                StorageAccess::Read => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_MEDIA_IMAGES".to_string(),
                        description: photos.description.clone(),
                    });
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSPhotoLibraryUsageDescription".to_string(),
                        value: photos.description.clone(),
                    });
                }
                StorageAccess::Write => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_EXTERNAL_STORAGE".to_string(),
                        description: photos.description.clone(),
                    });
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSPhotoLibraryAddUsageDescription".to_string(),
                        value: photos.description.clone(),
                    });
                }
                StorageAccess::ReadWrite => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_MEDIA_IMAGES".to_string(),
                        description: photos.description.clone(),
                    });
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_EXTERNAL_STORAGE".to_string(),
                        description: photos.description.clone(),
                    });
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSPhotoLibraryUsageDescription".to_string(),
                        value: photos.description.clone(),
                    });
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSPhotoLibraryAddUsageDescription".to_string(),
                        value: photos.description.clone(),
                    });
                }
            }

            self.macos_plist_entries.push(PlistEntry {
                key: "NSPhotoLibraryUsageDescription".to_string(),
                value: photos.description.clone(),
            });
        }
    }

    fn map_bluetooth(&mut self, permissions: &PermissionsConfig) {
        if let Some(bt) = &permissions.bluetooth {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.BLUETOOTH_CONNECT".to_string(),
                description: bt.description.clone(),
            });
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.BLUETOOTH_SCAN".to_string(),
                description: bt.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSBluetoothAlwaysUsageDescription".to_string(),
                value: bt.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSBluetoothAlwaysUsageDescription".to_string(),
                value: bt.description.clone(),
            });
        }
    }

    fn map_background_location(&mut self, permissions: &PermissionsConfig) {
        if let Some(bg_loc) = &permissions.background_location {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.ACCESS_BACKGROUND_LOCATION".to_string(),
                description: bg_loc.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSLocationAlwaysAndWhenInUseUsageDescription".to_string(),
                value: bg_loc.description.clone(),
            });
        }
    }

    fn map_contacts(&mut self, permissions: &PermissionsConfig) {
        if let Some(contacts) = &permissions.contacts {
            match contacts.access {
                StorageAccess::Read => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_CONTACTS".to_string(),
                        description: contacts.description.clone(),
                    });
                }
                StorageAccess::Write => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_CONTACTS".to_string(),
                        description: contacts.description.clone(),
                    });
                }
                StorageAccess::ReadWrite => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_CONTACTS".to_string(),
                        description: contacts.description.clone(),
                    });
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_CONTACTS".to_string(),
                        description: contacts.description.clone(),
                    });
                }
            }

            self.ios_plist_entries.push(PlistEntry {
                key: "NSContactsUsageDescription".to_string(),
                value: contacts.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSContactsUsageDescription".to_string(),
                value: contacts.description.clone(),
            });
        }
    }

    fn map_calendar(&mut self, permissions: &PermissionsConfig) {
        if let Some(cal) = &permissions.calendar {
            match cal.access {
                StorageAccess::Read => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_CALENDAR".to_string(),
                        description: cal.description.clone(),
                    });
                }
                StorageAccess::Write => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_CALENDAR".to_string(),
                        description: cal.description.clone(),
                    });
                }
                StorageAccess::ReadWrite => {
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.READ_CALENDAR".to_string(),
                        description: cal.description.clone(),
                    });
                    self.android_permissions.push(AndroidPermissionEntry {
                        permission: "android.permission.WRITE_CALENDAR".to_string(),
                        description: cal.description.clone(),
                    });
                }
            }

            self.ios_plist_entries.push(PlistEntry {
                key: "NSCalendarsUsageDescription".to_string(),
                value: cal.description.clone(),
            });

            self.macos_plist_entries.push(PlistEntry {
                key: "NSCalendarsUsageDescription".to_string(),
                value: cal.description.clone(),
            });
        }
    }

    fn map_biometrics(&mut self, permissions: &PermissionsConfig) {
        if let Some(bio) = &permissions.biometrics {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.USE_BIOMETRIC".to_string(),
                description: bio.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSFaceIDUsageDescription".to_string(),
                value: bio.description.clone(),
            });
        }
    }

    fn map_nfc(&mut self, permissions: &PermissionsConfig) {
        if let Some(nfc) = &permissions.nfc {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.NFC".to_string(),
                description: nfc.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NFCReaderUsageDescription".to_string(),
                value: nfc.description.clone(),
            });
        }
    }

    fn map_motion(&mut self, permissions: &PermissionsConfig) {
        if let Some(motion) = &permissions.motion {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.ACTIVITY_RECOGNITION".to_string(),
                description: motion.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSMotionUsageDescription".to_string(),
                value: motion.description.clone(),
            });
        }
    }

    fn map_health(&mut self, permissions: &PermissionsConfig) {
        if let Some(health) = &permissions.health {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.BODY_SENSORS".to_string(),
                description: health.description.clone(),
            });

            match health.access {
                StorageAccess::Read => {
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSHealthShareUsageDescription".to_string(),
                        value: health.description.clone(),
                    });
                }
                StorageAccess::Write => {
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSHealthUpdateUsageDescription".to_string(),
                        value: health.description.clone(),
                    });
                }
                StorageAccess::ReadWrite => {
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSHealthShareUsageDescription".to_string(),
                        value: health.description.clone(),
                    });
                    self.ios_plist_entries.push(PlistEntry {
                        key: "NSHealthUpdateUsageDescription".to_string(),
                        value: health.description.clone(),
                    });
                }
            }
        }
    }

    fn map_speech(&mut self, permissions: &PermissionsConfig) {
        if let Some(speech) = &permissions.speech {
            // Speech recognition uses microphone on Android
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.RECORD_AUDIO".to_string(),
                description: speech.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSSpeechRecognitionUsageDescription".to_string(),
                value: speech.description.clone(),
            });
        }
    }

    fn map_media_library(&mut self, permissions: &PermissionsConfig) {
        if let Some(media) = &permissions.media_library {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.READ_MEDIA_AUDIO".to_string(),
                description: media.description.clone(),
            });

            self.ios_plist_entries.push(PlistEntry {
                key: "NSAppleMusicUsageDescription".to_string(),
                value: media.description.clone(),
            });
        }
    }

    fn map_siri(&mut self, permissions: &PermissionsConfig) {
        if let Some(siri) = &permissions.siri {
            // Siri is iOS only
            self.ios_plist_entries.push(PlistEntry {
                key: "NSSiriUsageDescription".to_string(),
                value: siri.description.clone(),
            });
        }
    }

    fn map_homekit(&mut self, permissions: &PermissionsConfig) {
        if let Some(homekit) = &permissions.homekit {
            // HomeKit is iOS only
            self.ios_plist_entries.push(PlistEntry {
                key: "NSHomeKitUsageDescription".to_string(),
                value: homekit.description.clone(),
            });
        }
    }

    fn map_local_network(&mut self, permissions: &PermissionsConfig) {
        if let Some(network) = &permissions.local_network {
            // Local network is iOS only
            self.ios_plist_entries.push(PlistEntry {
                key: "NSLocalNetworkUsageDescription".to_string(),
                value: network.description.clone(),
            });
        }
    }

    fn map_nearby_wifi(&mut self, permissions: &PermissionsConfig) {
        if let Some(wifi) = &permissions.nearby_wifi {
            // Nearby WiFi is Android only
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.NEARBY_WIFI_DEVICES".to_string(),
                description: wifi.description.clone(),
            });
        }
    }

    /// Map deep link config to platform-specific URL schemes, associated domains, and intent filters
    fn map_deep_links(
        &mut self,
        deep_links: &DeepLinkConfig,
        android: &AndroidConfig,
        ios: &IosConfig,
        macos: &MacosConfig,
    ) {
        // Merge unified schemes with platform-specific overrides
        let mut ios_schemes: Vec<String> = deep_links.schemes.clone();
        ios_schemes.extend(ios.url_schemes.clone());
        ios_schemes.dedup();
        self.ios_url_schemes = ios_schemes;

        let mut macos_schemes: Vec<String> = deep_links.schemes.clone();
        macos_schemes.extend(macos.url_schemes.clone());
        macos_schemes.dedup();
        self.macos_url_schemes = macos_schemes;

        let mut android_schemes: Vec<String> = deep_links.schemes.clone();
        android_schemes.extend(android.url_schemes.clone());
        android_schemes.dedup();
        self.android_url_schemes = android_schemes;

        // Map universal link hosts to iOS associated domains
        for host in &deep_links.hosts {
            self.ios_associated_domains.push(format!("applinks:{host}"));
        }

        // Store app link hosts for Android auto-verify intent filters
        self.android_app_link_hosts = deep_links.hosts.clone();

        // Add explicit Android intent filters from config
        self.android_intent_filters = android.intent_filters.clone();
    }

    /// Map background mode config to platform-specific background capabilities
    fn map_background_modes(
        &mut self,
        background: &BackgroundConfig,
        android: &AndroidConfig,
        ios: &IosConfig,
    ) {
        // Build iOS UIBackgroundModes from unified config
        let mut ios_modes: Vec<String> = Vec::new();
        if background.location {
            ios_modes.push("location".to_string());
        }
        if background.audio {
            ios_modes.push("audio".to_string());
        }
        if background.fetch {
            ios_modes.push("fetch".to_string());
        }
        if background.remote_notifications {
            ios_modes.push("remote-notification".to_string());
        }
        if background.voip {
            ios_modes.push("voip".to_string());
        }
        if background.bluetooth {
            ios_modes.push("bluetooth-central".to_string());
            ios_modes.push("bluetooth-peripheral".to_string());
        }
        if background.external_accessory {
            ios_modes.push("external-accessory".to_string());
        }
        if background.processing {
            ios_modes.push("processing".to_string());
        }
        // Merge platform-specific overrides
        for mode in &ios.background_modes {
            if !ios_modes.contains(mode) {
                ios_modes.push(mode.clone());
            }
        }
        self.ios_background_modes = ios_modes;

        // Build Android foreground service types and permissions
        let mut android_types: Vec<String> = Vec::new();
        if background.location {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.ACCESS_BACKGROUND_LOCATION".to_string(),
                description: "Background location updates".to_string(),
            });
        }
        if background.audio {
            android_types.push("mediaPlayback".to_string());
        }
        if background.voip {
            android_types.push("phoneCall".to_string());
        }
        if background.bluetooth {
            android_types.push("connectedDevice".to_string());
        }
        // Merge platform-specific overrides
        for stype in &android.foreground_service_types {
            if !android_types.contains(stype) {
                android_types.push(stype.clone());
            }
        }
        // If we have any foreground service types, add the FOREGROUND_SERVICE permission
        if !android_types.is_empty() {
            self.android_permissions.push(AndroidPermissionEntry {
                permission: "android.permission.FOREGROUND_SERVICE".to_string(),
                description: "Run foreground services".to_string(),
            });
        }
        self.android_foreground_service_types = android_types;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LocationPermission, SimplePermission};

    #[test]
    fn test_location_permission_mapping() {
        let permissions = PermissionsConfig {
            location: Some(LocationPermission {
                precision: LocationPrecision::Fine,
                description: "Track your runs".to_string(),
            }),
            ..Default::default()
        };

        let mapper = ManifestMapper::from_config(
            &permissions,
            &DeepLinkConfig::default(),
            &BackgroundConfig::default(),
            &AndroidConfig::default(),
            &IosConfig::default(),
            &MacosConfig::default(),
        );

        // Should have both fine and coarse for Android
        assert!(mapper
            .android_permissions
            .iter()
            .any(|p| p.permission == "android.permission.ACCESS_FINE_LOCATION"));
        assert!(mapper
            .android_permissions
            .iter()
            .any(|p| p.permission == "android.permission.ACCESS_COARSE_LOCATION"));

        // Should have iOS location plist entry
        assert!(mapper
            .ios_plist_entries
            .iter()
            .any(|e| e.key == "NSLocationWhenInUseUsageDescription"));
    }

    #[test]
    fn test_camera_permission_mapping() {
        let permissions = PermissionsConfig {
            camera: Some(SimplePermission {
                description: "Take photos".to_string(),
            }),
            ..Default::default()
        };

        let mapper = ManifestMapper::from_config(
            &permissions,
            &DeepLinkConfig::default(),
            &BackgroundConfig::default(),
            &AndroidConfig::default(),
            &IosConfig::default(),
            &MacosConfig::default(),
        );

        assert!(mapper
            .android_permissions
            .iter()
            .any(|p| p.permission == "android.permission.CAMERA"));
        assert!(mapper
            .ios_plist_entries
            .iter()
            .any(|e| e.key == "NSCameraUsageDescription"));
    }

    #[test]
    fn test_android_camera_permission_data() {
        let permissions = PermissionsConfig {
            camera: Some(SimplePermission {
                description: "Take photos".to_string(),
            }),
            ..Default::default()
        };

        let mapper = ManifestMapper::from_config(
            &permissions,
            &DeepLinkConfig::default(),
            &BackgroundConfig::default(),
            &AndroidConfig::default(),
            &IosConfig::default(),
            &MacosConfig::default(),
        );

        assert!(mapper
            .android_permissions
            .iter()
            .any(|p| p.permission == "android.permission.CAMERA"));
    }
}
