//! Maps unified permissions from Dioxus.toml to platform-specific identifiers.
//!
//! This module converts the cross-platform permission declarations into:
//! - Android: `<uses-permission>` entries for AndroidManifest.xml
//! - iOS/macOS: Info.plist usage description keys

use crate::config::{
    AndroidConfig, IosConfig, LocationPrecision, MacosConfig, PermissionsConfig, StorageAccess,
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

/// Maps unified permissions to platform-specific identifiers
#[derive(Debug, Default)]
pub struct PermissionMapper {
    pub android_permissions: Vec<AndroidPermissionEntry>,
    pub android_features: Vec<String>,
    pub ios_plist_entries: Vec<PlistEntry>,
    pub macos_plist_entries: Vec<PlistEntry>,
}

impl PermissionMapper {
    /// Create a new permission mapper from the unified config
    pub fn from_config(
        permissions: &PermissionsConfig,
        android: &AndroidConfig,
        _ios: &IosConfig,
        _macos: &MacosConfig,
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

    /// Generate Android permission XML for AndroidManifest.xml
    pub fn generate_android_permissions_xml(&self) -> String {
        let mut xml = String::new();
        for perm in &self.android_permissions {
            xml.push_str(&format!(
                "    <uses-permission android:name=\"{}\" />\n",
                perm.permission
            ));
        }
        for feature in &self.android_features {
            xml.push_str(&format!(
                "    <uses-feature android:name=\"{}\" android:required=\"true\" />\n",
                feature
            ));
        }
        xml
    }

    /// Generate iOS plist XML for Info.plist
    pub fn generate_ios_plist_xml(&self) -> String {
        let mut xml = String::new();
        for entry in &self.ios_plist_entries {
            xml.push_str(&format!(
                "\t<key>{}</key>\n\t<string>{}</string>\n",
                entry.key, entry.value
            ));
        }
        xml
    }

    /// Generate macOS plist XML for Info.plist
    pub fn generate_macos_plist_xml(&self) -> String {
        let mut xml = String::new();
        for entry in &self.macos_plist_entries {
            xml.push_str(&format!(
                "\t<key>{}</key>\n\t<string>{}</string>\n",
                entry.key, entry.value
            ));
        }
        xml
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

        let mapper = PermissionMapper::from_config(
            &permissions,
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

        let mapper = PermissionMapper::from_config(
            &permissions,
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
    fn test_generate_android_xml() {
        let permissions = PermissionsConfig {
            camera: Some(SimplePermission {
                description: "Take photos".to_string(),
            }),
            ..Default::default()
        };

        let mapper = PermissionMapper::from_config(
            &permissions,
            &AndroidConfig::default(),
            &IosConfig::default(),
            &MacosConfig::default(),
        );

        let xml = mapper.generate_android_permissions_xml();
        assert!(xml.contains("android.permission.CAMERA"));
        assert!(xml.contains("uses-permission"));
    }
}
