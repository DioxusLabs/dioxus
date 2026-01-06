// Shared ActivityAttributes for Live Activities
// This MUST be the single source of truth for both the main app and widget extension

import Foundation
import ActivityKit

/// Live Activity attributes for displaying location permission status.
///
/// This struct is shared between the main app (which starts/updates activities)
/// and the widget extension (which renders them on the lock screen).
@available(iOS 16.2, *)
public struct LocationPermissionAttributes: ActivityAttributes {
    /// Dynamic content that can be updated while the activity is running
    public struct ContentState: Codable, Hashable {
        /// Current permission status: "prompt", "granted", "denied", "disabled", "unknown"
        public var permissionStatus: String
        /// Last update timestamp
        public var lastUpdated: Date

        public init(permissionStatus: String, lastUpdated: Date) {
            self.permissionStatus = permissionStatus
            self.lastUpdated = lastUpdated
        }
    }

    /// Static data set when the activity is started (cannot change)
    public var appName: String

    public init(appName: String) {
        self.appName = appName
    }
}
