// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

import CoreLocation
import Foundation
import Dispatch

/**
 * Simplified GeolocationPlugin for Dioxus that works without Tauri dependencies.
 * This can be shared with Tauri plugins with minimal changes.
 */
@objc(GeolocationPlugin)
public class GeolocationPlugin: NSObject, CLLocationManagerDelegate {
  private let locationManager = CLLocationManager()
  private var isUpdatingLocation: Bool = false
  private var positionCallbacks: [String: (String) -> Void] = [:]
  private var watcherCallbacks: [UInt32: (String) -> Void] = [:]
  private var permissionCallbacks: [String: (String) -> Void] = [:]

  override init() {
    super.init()
    locationManager.delegate = self
  }

  /**
   * Get current position as JSON string (called from ObjC/Rust)
   */
  @objc public func getCurrentPositionJson(_ optionsJson: String) -> String {
    // Parse options from JSON
    guard let optionsData = optionsJson.data(using: .utf8),
          let optionsDict = try? JSONSerialization.jsonObject(with: optionsData) as? [String: Any] else {
      let error = ["error": "Invalid options JSON"]
      return (try? JSONSerialization.data(withJSONObject: error))?.base64EncodedString() ?? ""
    }

    let enableHighAccuracy = optionsDict["enableHighAccuracy"] as? Bool ?? false
    let timeoutMs = optionsDict["timeout"] as? Double ?? 10000
    let maximumAgeMs = optionsDict["maximumAge"] as? Double ?? 0

    // If we have a recent cached location, return it immediately
    if let lastLocation = self.locationManager.location {
      let ageMs = Date().timeIntervalSince(lastLocation.timestamp) * 1000
      if maximumAgeMs <= 0 || ageMs <= maximumAgeMs {
        return self.convertLocationToJson(lastLocation)
      }
    }

    let callbackId = UUID().uuidString
    let semaphore = DispatchSemaphore(value: 0)
    var responseJson: String?

    self.positionCallbacks[callbackId] = { result in
      responseJson = result
      semaphore.signal()
    }

    if enableHighAccuracy {
      self.locationManager.desiredAccuracy = kCLLocationAccuracyBest
    } else {
      self.locationManager.desiredAccuracy = kCLLocationAccuracyKilometer
    }

    if CLLocationManager.authorizationStatus() == .notDetermined {
      self.locationManager.requestWhenInUseAuthorization()
    } else {
      self.locationManager.requestLocation()
    }

    let timeoutSeconds = max(timeoutMs / 1000.0, 0.1)
    let deadline = Date().addingTimeInterval(timeoutSeconds)
    while responseJson == nil && Date() < deadline {
      let _ = RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.05))
      if semaphore.wait(timeout: .now()) == .success {
        break
      }
    }

    if let json = responseJson {
      return json
    } else {
      // Timed out waiting for location
      self.positionCallbacks.removeValue(forKey: callbackId)
      let error = ["error": "Timeout waiting for location"]
      return (try? JSONSerialization.data(withJSONObject: error)).flatMap {
        String(data: $0, encoding: .utf8)
      } ?? "{\"error\":\"Timeout waiting for location\"}"
    }
  }

  /**
   * Watch position (called from ObjC/Rust)
   */
  @objc public func watchPositionNative(_ optionsJson: String, callbackId: UInt32) {
    guard let optionsData = optionsJson.data(using: .utf8),
          let optionsDict = try? JSONSerialization.jsonObject(with: optionsData) as? [String: Any] else {
      // Call error callback
      if let callback = self.watcherCallbacks[callbackId] {
        callback("{\"error\":\"Invalid options JSON\"}")
      }
      return
    }

    let enableHighAccuracy = optionsDict["enableHighAccuracy"] as? Bool ?? false

    self.watcherCallbacks[callbackId] = { result in
      // Will be called from delegate methods
    }

    DispatchQueue.main.async {
      if enableHighAccuracy {
        self.locationManager.desiredAccuracy = kCLLocationAccuracyBest
      } else {
        self.locationManager.desiredAccuracy = kCLLocationAccuracyKilometer
      }

      if CLLocationManager.authorizationStatus() == .notDetermined {
        self.locationManager.requestWhenInUseAuthorization()
      } else {
        self.locationManager.startUpdatingLocation()
        self.isUpdatingLocation = true
      }
    }
  }

  /**
   * Clear watch (called from ObjC/Rust)
   */
  @objc public func clearWatchNative(_ callbackId: UInt32) {
    self.watcherCallbacks.removeValue(forKey: callbackId)

    if self.watcherCallbacks.isEmpty {
      self.stopUpdating()
    }
  }

  /**
   * Check permissions and return JSON string (called from ObjC/Rust)
   */
  @objc public func checkPermissionsJson() -> String {
    var status: String = ""

    if CLLocationManager.locationServicesEnabled() {
      switch CLLocationManager.authorizationStatus() {
      case .notDetermined:
        status = "prompt"
      case .restricted, .denied:
        status = "denied"
      case .authorizedAlways, .authorizedWhenInUse:
        status = "granted"
      @unknown default:
        status = "prompt"
      }
    } else {
      let error = ["error": "Location services are not enabled"]
      return (try? JSONSerialization.data(withJSONObject: error))?.base64EncodedString() ?? ""
    }

    let result: [String: String] = ["location": status, "coarseLocation": status]

    if let jsonData = try? JSONSerialization.data(withJSONObject: result),
       let jsonString = String(data: jsonData, encoding: .utf8) {
      return jsonString
    }

    return ""
  }

  /**
   * Request permissions and return JSON string (called from ObjC/Rust)
   */
  @objc public func requestPermissionsJson(_ permissionsJson: String) -> String {
    if CLLocationManager.locationServicesEnabled() {
      if CLLocationManager.authorizationStatus() == .notDetermined {
        DispatchQueue.main.async {
          self.locationManager.requestWhenInUseAuthorization()
        }
        // Return current status - actual result comes via delegate
        return self.checkPermissionsJson()
      } else {
        return self.checkPermissionsJson()
      }
    } else {
      let error = ["error": "Location services are not enabled"]
      if let jsonData = try? JSONSerialization.data(withJSONObject: error),
         let jsonString = String(data: jsonData, encoding: .utf8) {
        return jsonString
      }
      return ""
    }
  }

  //
  // CLLocationManagerDelegate methods
  //

  public func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
    let errorMessage = error.localizedDescription

    // Notify all position callbacks
    for (_, callback) in self.positionCallbacks {
      let errorJson = "{\"error\":\"\(errorMessage)\"}"
      callback(errorJson)
    }
    self.positionCallbacks.removeAll()

    // Notify all watcher callbacks
    for (_, callback) in self.watcherCallbacks {
      let errorJson = "{\"error\":\"\(errorMessage)\"}"
      callback(errorJson)
    }
  }

  public func locationManager(
    _ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]
  ) {
    guard let location = locations.last else {
      return
    }

    let resultJson = self.convertLocationToJson(location)

    // Notify all position callbacks
    for (_, callback) in self.positionCallbacks {
      callback(resultJson)
    }
    self.positionCallbacks.removeAll()

    // Notify all watcher callbacks
    for (_, callback) in self.watcherCallbacks {
      callback(resultJson)
    }
  }

  public func locationManager(
    _ manager: CLLocationManager, didChangeAuthorization status: CLAuthorizationStatus
  ) {
    // Notify all permission callbacks
    let statusJson = self.checkPermissionsJson()
    for (_, callback) in self.permissionCallbacks {
      callback(statusJson)
    }
    self.permissionCallbacks.removeAll()

    if !self.positionCallbacks.isEmpty {
      self.locationManager.requestLocation()
    }

    if !self.watcherCallbacks.isEmpty && !self.isUpdatingLocation {
      self.locationManager.startUpdatingLocation()
      self.isUpdatingLocation = true
    }
  }

  //
  // Internal/Helper methods
  //

  private func stopUpdating() {
    self.locationManager.stopUpdatingLocation()
    self.isUpdatingLocation = false
  }

  private func convertLocationToJson(_ location: CLLocation) -> String {
    var ret: [String: Any] = [:]
    var coords: [String: Any] = [:]

    coords["latitude"] = location.coordinate.latitude
    coords["longitude"] = location.coordinate.longitude
    coords["accuracy"] = location.horizontalAccuracy
    coords["altitude"] = location.altitude
    coords["altitudeAccuracy"] = location.verticalAccuracy
    coords["speed"] = location.speed
    coords["heading"] = location.course
    ret["timestamp"] = Int((location.timestamp.timeIntervalSince1970 * 1000))
    ret["coords"] = coords

    if let jsonData = try? JSONSerialization.data(withJSONObject: ret),
       let jsonString = String(data: jsonData, encoding: .utf8) {
      return jsonString
    }

    return "{\"error\":\"Failed to serialize location\"}"
  }

  /**
   * Callback functions to be called from Rust
   * These are declared as external functions implemented in Rust
   */
  @objc public func onLocationUpdateNative(_ watchId: UInt32, locationJson: String) {
    // This will be called from Rust when location updates arrive
    // The Rust code will handle the actual callback invocation
    dioxus_geolocation_on_location_update(watchId, locationJson)
  }

  @objc public func onLocationErrorNative(_ watchId: UInt32, errorMessage: String) {
    // This will be called from Rust when location errors occur
    dioxus_geolocation_on_location_error(watchId, errorMessage)
  }
}

/**
 * Anchor function to force the Swift object file into linked binaries.
 * Rust calls this symbol at startup to ensure the class is registered with the ObjC runtime.
 */
@_cdecl("dioxus_geolocation_plugin_init")
public func dioxus_geolocation_plugin_init() {
  _ = GeolocationPlugin.self
}

/**
 * External functions declared in Rust
 * These will be implemented in the Rust code via objc2 bindings
 */
@_cdecl("dioxus_geolocation_on_location_update")
func dioxus_geolocation_on_location_update(_ watchId: UInt32, _ locationJson: UnsafePointer<CChar>) {
  // This is a placeholder - the actual implementation is in Rust
  // The Swift code will call this from the delegate methods
}

@_cdecl("dioxus_geolocation_on_location_error")
func dioxus_geolocation_on_location_error(_ watchId: UInt32, _ errorMessage: UnsafePointer<CChar>) {
  // This is a placeholder - the actual implementation is in Rust
  // The Swift code will call this from the delegate methods
}
