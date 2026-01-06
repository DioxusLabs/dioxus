// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

import CoreLocation
import Foundation
import Dispatch
import ActivityKit

/**
 * Simplified GeolocationPlugin for Dioxus that works without Tauri dependencies.
 * This can be shared with Tauri plugins with minimal changes.
 */
@objc(GeolocationPlugin)
public class GeolocationPlugin: NSObject, CLLocationManagerDelegate {
  private let locationManager = CLLocationManager()
  private var positionCallbacks: [String: (String) -> Void] = [:]

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

  }

  public func locationManager(
    _ manager: CLLocationManager, didChangeAuthorization status: CLAuthorizationStatus
  ) {
    if !self.positionCallbacks.isEmpty {
      self.locationManager.requestLocation()
    }
  }

  //
  // Internal/Helper methods
  //

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

  //
  // Live Activity methods
  //

  /// Start a Live Activity showing permission status
  /// Returns JSON with activity ID or error
  @objc public func startLiveActivityJson() -> String {
    if #available(iOS 16.2, *) {
      // Check if Live Activities are enabled
      guard ActivityAuthorizationInfo().areActivitiesEnabled else {
        return "{\"error\":\"Live Activities are not enabled\"}"
      }

      // Get current permission status
      let status = self.getCurrentPermissionStatus()

      let attributes = LocationPermissionAttributes(appName: "Geolocation Demo")
      let contentState = LocationPermissionAttributes.ContentState(
        permissionStatus: status,
        lastUpdated: Date()
      )

      do {
        let activity = try Activity.request(
          attributes: attributes,
          content: .init(state: contentState, staleDate: nil),
          pushType: nil
        )

        // Store reference (note: this won't persist across method calls in current setup)
        // In a real app, you'd store the activity ID and look it up

        let result: [String: Any] = [
          "activityId": activity.id,
          "permissionStatus": status
        ]

        if let jsonData = try? JSONSerialization.data(withJSONObject: result),
           let jsonString = String(data: jsonData, encoding: .utf8) {
          return jsonString
        }
        return "{\"error\":\"Failed to serialize result\"}"
      } catch {
        return "{\"error\":\"Failed to start Live Activity: \(error.localizedDescription)\"}"
      }
    } else {
      return "{\"error\":\"Live Activities require iOS 16.2+\"}"
    }
  }

  /// Update the Live Activity with new permission status
  /// Takes JSON with activityId
  @objc public func updateLiveActivityJson(_ statusJson: String) -> String {
    if #available(iOS 16.2, *) {
      // Get current permission status
      let status = self.getCurrentPermissionStatus()

      let contentState = LocationPermissionAttributes.ContentState(
        permissionStatus: status,
        lastUpdated: Date()
      )

      // Update all running activities of this type
      Task {
        for activity in Activity<LocationPermissionAttributes>.activities {
          await activity.update(
            ActivityContent(state: contentState, staleDate: nil)
          )
        }
      }

      let result: [String: String] = ["permissionStatus": status]
      if let jsonData = try? JSONSerialization.data(withJSONObject: result),
         let jsonString = String(data: jsonData, encoding: .utf8) {
        return jsonString
      }
      return "{\"error\":\"Failed to serialize result\"}"
    } else {
      return "{\"error\":\"Live Activities require iOS 16.2+\"}"
    }
  }

  /// End all Live Activities
  @objc public func endLiveActivityJson() -> String {
    if #available(iOS 16.2, *) {
      Task {
        for activity in Activity<LocationPermissionAttributes>.activities {
          await activity.end(nil, dismissalPolicy: .immediate)
        }
      }
      return "{\"success\":true}"
    } else {
      return "{\"error\":\"Live Activities require iOS 16.2+\"}"
    }
  }

  /// Helper to get current permission status string
  private func getCurrentPermissionStatus() -> String {
    if CLLocationManager.locationServicesEnabled() {
      switch CLLocationManager.authorizationStatus() {
      case .notDetermined:
        return "prompt"
      case .restricted, .denied:
        return "denied"
      case .authorizedAlways, .authorizedWhenInUse:
        return "granted"
      @unknown default:
        return "unknown"
      }
    } else {
      return "disabled"
    }
  }

}
