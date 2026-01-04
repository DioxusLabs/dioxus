// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

package com.dioxus.geolocation

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.location.Location
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.Timer
import kotlin.concurrent.schedule

class GeolocationPlugin(private val activity: Activity) {
  private val geolocation = Geolocation(activity)

  fun checkPermissions(): Map<String, String> {
    val response = mutableMapOf<String, String>()
    val coarseStatus = ContextCompat.checkSelfPermission(activity, Manifest.permission.ACCESS_COARSE_LOCATION)
    val fineStatus = ContextCompat.checkSelfPermission(activity, Manifest.permission.ACCESS_FINE_LOCATION)

    response["location"] = permissionToStatus(fineStatus)
    response["coarseLocation"] = permissionToStatus(coarseStatus)

    return response
  }

  fun requestPermissions(callback: (Map<String, String>) -> Unit) {
    val permissionsToRequest = mutableListOf<String>()

    if (ContextCompat.checkSelfPermission(activity, Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
      permissionsToRequest.add(Manifest.permission.ACCESS_FINE_LOCATION)
    }

    if (ContextCompat.checkSelfPermission(activity, Manifest.permission.ACCESS_COARSE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
      permissionsToRequest.add(Manifest.permission.ACCESS_COARSE_LOCATION)
    }

    if (permissionsToRequest.isEmpty()) {
      callback(checkPermissions())
    } else {
      ActivityCompat.requestPermissions(activity, permissionsToRequest.toTypedArray(), 1001)
      Handler(Looper.getMainLooper()).postDelayed({ callback(checkPermissions()) }, 1000)
    }
  }

  fun getCurrentPosition(
    enableHighAccuracy: Boolean,
    timeout: Long,
    maximumAge: Long,
    successCallback: (Location) -> Unit,
    errorCallback: (String) -> Unit,
  ) {
    val lastLocation = geolocation.getLastLocation(maximumAge)
    if (lastLocation != null) {
      successCallback(lastLocation)
      return
    }

    val timer = Timer()
    timer.schedule(timeout) {
      activity.runOnUiThread { errorCallback("Timeout waiting for location.") }
    }

    geolocation.sendLocation(
      enableHighAccuracy,
      { location ->
        timer.cancel()
        successCallback(location)
      },
      { error ->
        timer.cancel()
        errorCallback(error)
      },
    )
  }

  private fun permissionToStatus(value: Int): String =
    when (value) {
      PackageManager.PERMISSION_GRANTED -> "granted"
      PackageManager.PERMISSION_DENIED -> "denied"
      else -> "prompt"
    }

  // ---- Platform bridge helpers expected by Rust JNI layer ----

  // Called by Rust after constructing the plugin. No-op placeholder to match signature.
  fun load(webView: WebView?) { /* no-op */ }

  // Serialize current permission status as JSON string
  fun checkPermissionsJson(): String {
    val status = checkPermissions()
    val json = JSONObject()
    json.put("location", status["location"]) // granted|denied|prompt
    json.put("coarseLocation", status["coarseLocation"]) // granted|denied|prompt
    return json.toString()
  }

  // Request permissions and return resulting status JSON (waits briefly for result)
  fun requestPermissionsJson(permissionsJson: String?): String {
    val latch = CountDownLatch(1)
    var result: String = checkPermissionsJson()

    requestPermissions { status ->
      val json = JSONObject()
      json.put("location", status["location"])
      json.put("coarseLocation", status["coarseLocation"])
      result = json.toString()
      latch.countDown()
    }

    // Wait up to 5 seconds for the permission result, then return whatever we have
    latch.await(5, TimeUnit.SECONDS)
    return result
  }

  // Convert a Location to the Position JSON expected by Rust side
  private fun locationToPositionJson(location: Location): String {
    val coords = JSONObject()
    coords.put("latitude", location.latitude)
    coords.put("longitude", location.longitude)
    coords.put("accuracy", location.accuracy.toDouble())
    if (location.hasAltitude()) coords.put("altitude", location.altitude)
    if (android.os.Build.VERSION.SDK_INT >= 26) {
      val vAcc = try { location.verticalAccuracyMeters } catch (_: Exception) { null }
      if (vAcc != null) coords.put("altitudeAccuracy", vAcc.toDouble())
    }
    if (location.hasSpeed()) coords.put("speed", location.speed.toDouble())
    if (location.hasBearing()) coords.put("heading", location.bearing.toDouble())

    val obj = JSONObject()
    obj.put("timestamp", System.currentTimeMillis())
    obj.put("coords", coords)
    return obj.toString()
  }

  // Synchronous wrapper returning JSON for getCurrentPosition
  // Accepts a JSON string with options: {"enableHighAccuracy": bool, "timeout": number, "maximumAge": number}
  fun getCurrentPositionJson(optionsJson: String?): String {
    val options = try {
      if (optionsJson.isNullOrEmpty()) JSONObject() else JSONObject(optionsJson)
    } catch (e: Exception) {
      JSONObject()
    }
    val enableHighAccuracy = options.optBoolean("enableHighAccuracy", false)
    val timeout = options.optLong("timeout", 10000L)
    val maximumAge = options.optLong("maximumAge", 0L)

    var output: String? = null
    val latch = CountDownLatch(1)

    getCurrentPosition(
      enableHighAccuracy,
      timeout,
      maximumAge,
      { location ->
        output = locationToPositionJson(location)
        latch.countDown()
      },
      { error ->
        output = JSONObject(mapOf("error" to error)).toString()
        latch.countDown()
      },
    )

    // Wait up to the timeout + 2s buffer
    latch.await(timeout + 2000, TimeUnit.MILLISECONDS)
    return output ?: JSONObject(mapOf("error" to "Timeout waiting for location.")).toString()
  }

}
