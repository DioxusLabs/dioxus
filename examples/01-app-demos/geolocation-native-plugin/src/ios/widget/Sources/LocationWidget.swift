// Widget Extension for Live Activity only
// Note: Having multiple widgets in the same bundle can cause Live Activities to show black
// See: https://developer.apple.com/forums/thread/807726
import ActivityKit
import SwiftUI
import WidgetKit

@main
struct LocationWidgetBundle: WidgetBundle {
    var body: some Widget {
        // Only include the Live Activity - other widgets can cause rendering issues
        LocationPermissionLiveActivity()
    }
}

// Live Activity widget
struct LocationPermissionLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: LocationPermissionAttributes.self) { context in
            // Lock screen view - use explicit VStack with background
            VStack {
                HStack {
                    Image(systemName: "location.fill")
                        .foregroundColor(.green)
                        .font(.title2)

                    VStack(alignment: .leading) {
                        Text(context.attributes.appName)
                            .font(.headline)
                            .foregroundColor(.white)
                        Text("Permission: \(context.state.permissionStatus)")
                            .font(.subheadline)
                            .foregroundColor(.cyan)
                    }

                    Spacer()
                }
                .padding()
            }
            .activityBackgroundTint(Color.black.opacity(0.8))
            .activitySystemActionForegroundColor(.white)

        } dynamicIsland: { context in
            DynamicIsland {
                // Expanded regions
                DynamicIslandExpandedRegion(.leading) {
                    Image(systemName: "location.fill")
                        .foregroundColor(.green)
                        .font(.title2)
                }

                DynamicIslandExpandedRegion(.center) {
                    VStack {
                        Text(context.attributes.appName)
                            .font(.headline)
                        Text(context.state.permissionStatus)
                            .font(.caption)
                            .foregroundColor(.cyan)
                    }
                }

                DynamicIslandExpandedRegion(.trailing) {
                    Text(context.state.permissionStatus.prefix(3).uppercased())
                        .font(.caption)
                        .foregroundColor(.green)
                }

            } compactLeading: {
                Image(systemName: "location.fill")
                    .foregroundColor(.green)
            } compactTrailing: {
                Text(context.state.permissionStatus.prefix(3).uppercased())
                    .font(.caption2)
                    .foregroundColor(.green)
            } minimal: {
                Image(systemName: "location.fill")
                    .foregroundColor(.green)
            }
        }
    }
}
