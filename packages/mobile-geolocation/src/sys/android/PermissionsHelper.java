package dioxus.mobile.geolocation;

import android.app.Activity;

/**
 * Utility to ensure permission requests execute on the main thread.
 */
public final class PermissionsHelper {
    private PermissionsHelper() {}

    public static void requestPermissionsOnUiThread(
            final Activity activity,
            final String[] permissions,
            final int requestCode
    ) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                activity.requestPermissions(permissions, requestCode);
            }
        });
    }
}
