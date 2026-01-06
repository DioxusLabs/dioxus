// Widget Extension for displaying location permission status
// This provides the lock screen UI for the Live Activity started by GeolocationPlugin

import ActivityKit
import SwiftUI
import WidgetKit

/// Live Activity attributes - must match the definition in GeolocationPlugin.swift
struct LocationPermissionAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        /// Current permission status: "prompt", "granted", "denied"
        var permissionStatus: String
        /// Last update timestamp
        var lastUpdated: Date
    }

    /// App name to display
    var appName: String
}

/// The main widget bundle that contains all our widgets
@main
struct LocationWidgetBundle: WidgetBundle {
    var body: some Widget {
        // Live Activity widget
        LocationPermissionLiveActivity()
    }
}

/// Live Activity widget configuration
struct LocationPermissionLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: LocationPermissionAttributes.self) { context in
            // Lock screen / notification banner view
            LockScreenView(context: context)
        } dynamicIsland: { context in
            // Dynamic Island configuration (for iPhone 14 Pro and later)
            DynamicIsland {
                // Expanded view
                DynamicIslandExpandedRegion(.leading) {
                    Image(systemName: "location.fill")
                        .foregroundColor(statusColor(context.state.permissionStatus))
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Text(statusEmoji(context.state.permissionStatus))
                        .font(.title)
                }
                DynamicIslandExpandedRegion(.center) {
                    Text("Location Permission")
                        .font(.headline)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    HStack {
                        Text("Status:")
                            .foregroundColor(.secondary)
                        Text(context.state.permissionStatus.capitalized)
                            .fontWeight(.semibold)
                            .foregroundColor(statusColor(context.state.permissionStatus))
                    }
                }
            } compactLeading: {
                Image(systemName: "location.fill")
                    .foregroundColor(statusColor(context.state.permissionStatus))
            } compactTrailing: {
                Text(statusEmoji(context.state.permissionStatus))
            } minimal: {
                Image(systemName: "location.fill")
                    .foregroundColor(statusColor(context.state.permissionStatus))
            }
        }
    }
}

/// Lock screen view for the Live Activity
struct LockScreenView: View {
    let context: ActivityViewContext<LocationPermissionAttributes>

    var body: some View {
        HStack(spacing: 16) {
            // Status icon
            ZStack {
                Circle()
                    .fill(statusColor(context.state.permissionStatus).opacity(0.2))
                    .frame(width: 50, height: 50)
                Image(systemName: statusIcon(context.state.permissionStatus))
                    .font(.title2)
                    .foregroundColor(statusColor(context.state.permissionStatus))
            }

            // Info
            VStack(alignment: .leading, spacing: 4) {
                Text(context.attributes.appName)
                    .font(.headline)
                HStack {
                    Text("Location:")
                        .foregroundColor(.secondary)
                    Text(context.state.permissionStatus.capitalized)
                        .fontWeight(.semibold)
                        .foregroundColor(statusColor(context.state.permissionStatus))
                }
                .font(.subheadline)
            }

            Spacer()

            // Status emoji
            Text(statusEmoji(context.state.permissionStatus))
                .font(.largeTitle)
        }
        .padding()
        .activityBackgroundTint(Color.black.opacity(0.8))
    }
}

// MARK: - Helper Functions

func statusColor(_ status: String) -> Color {
    switch status.lowercased() {
    case "granted":
        return .green
    case "denied", "disabled":
        return .red
    case "prompt":
        return .orange
    default:
        return .gray
    }
}

func statusIcon(_ status: String) -> String {
    switch status.lowercased() {
    case "granted":
        return "location.fill"
    case "denied", "disabled":
        return "location.slash.fill"
    case "prompt":
        return "location.circle"
    default:
        return "questionmark.circle"
    }
}

func statusEmoji(_ status: String) -> String {
    switch status.lowercased() {
    case "granted":
        return "✅"
    case "denied", "disabled":
        return "❌"
    case "prompt":
        return "❓"
    default:
        return "❔"
    }
}
