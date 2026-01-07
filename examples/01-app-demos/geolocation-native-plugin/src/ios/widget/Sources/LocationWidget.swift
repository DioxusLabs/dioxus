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

// Helper to get accuracy color
func accuracyColor(_ accuracy: Double) -> Color {
    if accuracy < 10 {
        return .green
    } else if accuracy < 50 {
        return .yellow
    } else if accuracy < 100 {
        return .orange
    } else {
        return .red
    }
}

// Helper to format coordinates with degree symbol
func formatCoord(_ value: Double, isLat: Bool) -> String {
    let direction = isLat ? (value >= 0 ? "N" : "S") : (value >= 0 ? "E" : "W")
    return String(format: "%.5f° %@", abs(value), direction)
}

// Live Activity widget
struct LocationPermissionLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: LocationPermissionAttributes.self) { context in
            // Lock screen view - flashy gradient design
            ZStack {
                // Gradient background
                LinearGradient(
                    colors: [
                        Color(red: 0.1, green: 0.1, blue: 0.2),
                        Color(red: 0.05, green: 0.15, blue: 0.25)
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )

                VStack(spacing: 12) {
                    // Header with pulsing indicator
                    HStack(spacing: 12) {
                        // Animated location icon with glow
                        ZStack {
                            Circle()
                                .fill(accuracyColor(context.state.accuracy).opacity(0.3))
                                .frame(width: 44, height: 44)
                            Circle()
                                .fill(accuracyColor(context.state.accuracy).opacity(0.6))
                                .frame(width: 32, height: 32)
                            Image(systemName: "location.fill")
                                .font(.system(size: 18, weight: .bold))
                                .foregroundColor(.white)
                        }

                        VStack(alignment: .leading, spacing: 2) {
                            Text(context.attributes.appName)
                                .font(.headline)
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                            HStack(spacing: 4) {
                                Image(systemName: "antenna.radiowaves.left.and.right")
                                    .font(.caption2)
                                Text("LIVE")
                                    .font(.caption2)
                                    .fontWeight(.bold)
                            }
                            .foregroundColor(accuracyColor(context.state.accuracy))
                        }

                        Spacer()

                        // Accuracy with animated ring
                        VStack(spacing: 2) {
                            Text("\(Int(context.state.accuracy))")
                                .font(.system(size: 24, weight: .bold, design: .rounded))
                                .foregroundColor(accuracyColor(context.state.accuracy))
                                .contentTransition(.numericText())
                            Text("meters")
                                .font(.caption2)
                                .foregroundColor(.gray)
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 8)
                        .background(
                            RoundedRectangle(cornerRadius: 12)
                                .fill(accuracyColor(context.state.accuracy).opacity(0.15))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 12)
                                        .strokeBorder(accuracyColor(context.state.accuracy).opacity(0.5), lineWidth: 1)
                                )
                        )
                    }

                    // Coordinates in stylish cards
                    HStack(spacing: 8) {
                        // Latitude card
                        VStack(alignment: .leading, spacing: 4) {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.up.arrow.down")
                                    .font(.caption2)
                                Text("LATITUDE")
                                    .font(.caption2)
                                    .fontWeight(.semibold)
                            }
                            .foregroundColor(.cyan.opacity(0.8))

                            Text(formatCoord(context.state.latitude, isLat: true))
                                .font(.system(.callout, design: .monospaced))
                                .fontWeight(.medium)
                                .foregroundColor(.white)
                                .contentTransition(.numericText())
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(10)
                        .background(
                            RoundedRectangle(cornerRadius: 10)
                                .fill(Color.cyan.opacity(0.1))
                        )

                        // Longitude card
                        VStack(alignment: .leading, spacing: 4) {
                            HStack(spacing: 4) {
                                Image(systemName: "arrow.left.arrow.right")
                                    .font(.caption2)
                                Text("LONGITUDE")
                                    .font(.caption2)
                                    .fontWeight(.semibold)
                            }
                            .foregroundColor(.purple.opacity(0.8))

                            Text(formatCoord(context.state.longitude, isLat: false))
                                .font(.system(.callout, design: .monospaced))
                                .fontWeight(.medium)
                                .foregroundColor(.white)
                                .contentTransition(.numericText())
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(10)
                        .background(
                            RoundedRectangle(cornerRadius: 10)
                                .fill(Color.purple.opacity(0.1))
                        )
                    }

                    // Speed and heading row (if available)
                    if let speed = context.state.speed, speed >= 0 {
                        HStack(spacing: 16) {
                            // Speed
                            HStack(spacing: 6) {
                                Image(systemName: "speedometer")
                                    .foregroundColor(.orange)
                                Text(String(format: "%.1f m/s", speed))
                                    .font(.system(.caption, design: .monospaced))
                                    .fontWeight(.medium)
                                    .foregroundColor(.white)
                                    .contentTransition(.numericText())
                            }

                            // Heading if available
                            if let heading = context.state.heading, heading >= 0 {
                                HStack(spacing: 6) {
                                    Image(systemName: "safari")
                                        .foregroundColor(.mint)
                                        .rotationEffect(.degrees(heading))
                                    Text(String(format: "%.0f°", heading))
                                        .font(.system(.caption, design: .monospaced))
                                        .fontWeight(.medium)
                                        .foregroundColor(.white)
                                        .contentTransition(.numericText())
                                }
                            }

                            Spacer()

                            // Last updated
                            Text(context.state.lastUpdated, style: .relative)
                                .font(.caption2)
                                .foregroundColor(.gray)
                        }
                    }
                }
                .padding()
            }
            .activitySystemActionForegroundColor(.white)

        } dynamicIsland: { context in
            DynamicIsland {
                // Expanded regions
                DynamicIslandExpandedRegion(.leading) {
                    VStack(alignment: .leading, spacing: 4) {
                        ZStack {
                            Circle()
                                .fill(accuracyColor(context.state.accuracy).opacity(0.3))
                                .frame(width: 36, height: 36)
                            Image(systemName: "location.fill")
                                .foregroundColor(accuracyColor(context.state.accuracy))
                                .font(.system(size: 16, weight: .bold))
                        }
                        Text("\(Int(context.state.accuracy))m")
                            .font(.caption2)
                            .fontWeight(.bold)
                            .foregroundColor(accuracyColor(context.state.accuracy))
                            .contentTransition(.numericText())
                    }
                }

                DynamicIslandExpandedRegion(.center) {
                    VStack(spacing: 6) {
                        // Latitude
                        HStack(spacing: 4) {
                            Text("LAT")
                                .font(.caption2)
                                .foregroundColor(.cyan.opacity(0.7))
                            Text(String(format: "%.5f°", context.state.latitude))
                                .font(.system(.caption, design: .monospaced))
                                .fontWeight(.semibold)
                                .foregroundColor(.cyan)
                                .contentTransition(.numericText())
                        }
                        // Longitude
                        HStack(spacing: 4) {
                            Text("LON")
                                .font(.caption2)
                                .foregroundColor(.purple.opacity(0.7))
                            Text(String(format: "%.5f°", context.state.longitude))
                                .font(.system(.caption, design: .monospaced))
                                .fontWeight(.semibold)
                                .foregroundColor(.purple)
                                .contentTransition(.numericText())
                        }
                    }
                }

                DynamicIslandExpandedRegion(.trailing) {
                    if let speed = context.state.speed, speed >= 0 {
                        VStack(alignment: .trailing, spacing: 4) {
                            Image(systemName: "speedometer")
                                .foregroundColor(.orange)
                                .font(.caption)
                            Text(String(format: "%.1f", speed))
                                .font(.system(.caption, design: .rounded))
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                                .contentTransition(.numericText())
                            Text("m/s")
                                .font(.caption2)
                                .foregroundColor(.gray)
                        }
                    } else {
                        // Show heading compass if no speed
                        if let heading = context.state.heading, heading >= 0 {
                            VStack(spacing: 2) {
                                Image(systemName: "safari")
                                    .font(.title3)
                                    .foregroundColor(.mint)
                                    .rotationEffect(.degrees(heading))
                                Text(String(format: "%.0f°", heading))
                                    .font(.caption2)
                                    .foregroundColor(.white)
                            }
                        }
                    }
                }

                DynamicIslandExpandedRegion(.bottom) {
                    HStack {
                        Text(context.attributes.appName)
                            .font(.caption2)
                            .foregroundColor(.gray)
                        Spacer()
                        Text(context.state.lastUpdated, style: .relative)
                            .font(.caption2)
                            .foregroundColor(.gray)
                    }
                }

            } compactLeading: {
                ZStack {
                    Circle()
                        .fill(accuracyColor(context.state.accuracy).opacity(0.3))
                        .frame(width: 24, height: 24)
                    Image(systemName: "location.fill")
                        .foregroundColor(accuracyColor(context.state.accuracy))
                        .font(.caption)
                }
            } compactTrailing: {
                Text(String(format: "%.4f°", context.state.latitude))
                    .font(.system(.caption2, design: .monospaced))
                    .fontWeight(.medium)
                    .foregroundColor(.cyan)
                    .contentTransition(.numericText())
            } minimal: {
                ZStack {
                    Circle()
                        .fill(accuracyColor(context.state.accuracy).opacity(0.3))
                        .frame(width: 22, height: 22)
                    Image(systemName: "location.fill")
                        .foregroundColor(accuracyColor(context.state.accuracy))
                        .font(.system(size: 10, weight: .bold))
                }
            }
        }
    }
}
