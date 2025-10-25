/* This file is compiled by build.rs */

package dioxus.mobile.geolocation;

import android.location.Location;
import android.location.LocationListener;
import java.util.function.Consumer;
import java.util.List;

/**
 * Callback class for location updates.
 * 
 * Implements both Consumer<Location> for getCurrentLocation
 * and LocationListener for requestLocationUpdates.
 */
public class LocationCallback implements Consumer<Location>, LocationListener {
    private long handlerPtrHigh;
    private long handlerPtrLow;
    private boolean executing;
    private boolean doNotExecute;

    /**
     * The name and signature of this function must be kept in sync with 
     * RUST_CALLBACK_NAME and RUST_CALLBACK_SIGNATURE in callback.rs
     */
    private native void rustCallback(long handlerPtrHigh, long handlerPtrLow, Location location);

    public LocationCallback(long handlerPtrHigh, long handlerPtrLow) {
        this.handlerPtrHigh = handlerPtrHigh;
        this.handlerPtrLow = handlerPtrLow;
        this.executing = false;
        this.doNotExecute = false;
    }

    public boolean isExecuting() {
        return this.executing;
    }

    public void disableExecution() {
        this.doNotExecute = true;
    }

    @Override
    public void accept(Location location) {
        this.executing = true;
        if (!this.doNotExecute) {
            rustCallback(this.handlerPtrHigh, this.handlerPtrLow, location);
        }
        this.executing = false;
    }

    @Override
    public void onLocationChanged(Location location) {
        this.executing = true;
        if (!this.doNotExecute) {
            rustCallback(this.handlerPtrHigh, this.handlerPtrLow, location);
        }
        this.executing = false;
    }

    /**
     * NOTE: Technically implementing this function shouldn't be necessary as it has 
     * a default implementation, but if we don't we get the following error:
     * NoClassDefFoundError for android/location/LocationListener$-CC
     */
    @Override
    public void onLocationChanged(List<Location> locations) {
        this.executing = true;
        if (!this.doNotExecute) {
            for (Location location : locations) {
                rustCallback(this.handlerPtrHigh, this.handlerPtrLow, location);
            }
        }
        this.executing = false;
    }
}
