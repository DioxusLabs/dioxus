// swift-tools-version:5.9
// Widget Extension for displaying location permission status on lock screen
//
// IMPORTANT: The target name MUST match the main app's Swift module name.
// The plugin uses "GeolocationPlugin" as its package/target name, so the
// widget must also use "GeolocationPlugin" for ActivityKit type matching.

import PackageDescription

let package = Package(
    name: "GeolocationPlugin",
    platforms: [
        .iOS(.v17),  // iOS 17+ for latest ActivityKit APIs
    ],
    products: [
        // Executable name must be "widget" for the build system to find it
        // But the TARGET name determines the Swift module name
        .executable(
            name: "widget",
            targets: ["GeolocationPlugin"]
        )
    ],
    dependencies: [],
    targets: [
        // Target name = Swift module name = "GeolocationPlugin"
        // This MUST match the main app's Swift plugin module name!
        .executableTarget(
            name: "GeolocationPlugin",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("WidgetKit"),
                .linkedFramework("SwiftUI"),
                .linkedFramework("ActivityKit"),
            ]
        )
    ]
)
