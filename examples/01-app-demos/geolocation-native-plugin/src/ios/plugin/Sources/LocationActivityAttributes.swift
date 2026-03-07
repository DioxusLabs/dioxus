// Shared ActivityAttributes for Live Activities
// This MUST be the single source of truth for both the main app and widget extension

import Foundation
import ActivityKit

/// Live Activity attributes for displaying current location.
///
/// This struct is shared between the main app (which starts/updates activities)
/// and the widget extension (which renders them on the lock screen).
public struct LocationPermissionAttributes: ActivityAttributes {
    /// Dynamic content that can be updated while the activity is running
    public struct ContentState: Codable, Hashable {
        /// Current latitude
        public var latitude: Double
        /// Current longitude
        public var longitude: Double
        /// Horizontal accuracy in meters
        public var accuracy: Double
        /// Current speed in m/s (nil if not available)
        public var speed: Double?
        /// Current heading in degrees (nil if not available)
        public var heading: Double?
        /// Last update timestamp
        public var lastUpdated: Date

        public init(
            latitude: Double,
            longitude: Double,
            accuracy: Double,
            speed: Double? = nil,
            heading: Double? = nil,
            lastUpdated: Date
        ) {
            self.latitude = latitude
            self.longitude = longitude
            self.accuracy = accuracy
            self.speed = speed
            self.heading = heading
            self.lastUpdated = lastUpdated
        }
    }

    /// Static data set when the activity is started (cannot change)
    public var appName: String

    public init(appName: String) {
        self.appName = appName
    }
}
