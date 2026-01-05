// swift-tools-version:5.7
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

import PackageDescription

let package = Package(
  name: "GeolocationPlugin",
  platforms: [
    .iOS(.v16),  // iOS 16.1+ required for Live Activities
    .macOS(.v13),
  ],
  products: [
    .library(
      name: "GeolocationPlugin",
      type: .static,
      targets: ["GeolocationPlugin"])
  ],
  dependencies: [],
  targets: [
    .target(
      name: "GeolocationPlugin",
      path: "Sources",
      linkerSettings: [
        .linkedFramework("CoreLocation"),
        .linkedFramework("Foundation"),
        .linkedFramework("ActivityKit", .when(platforms: [.iOS])),
      ])
  ]
)
