
APP_PATH="target/aarch64-apple-ios/debug/bundle/ios/DioxusApp.app"

# get the device id by jq-ing the json of the device list
xcrun devicectl list devices --json-output target/deviceid.json
DEVICE_UUID=$(jq -r '.result.devices[0].identifier' target/deviceid.json)

xcrun devicectl device install app --device "${DEVICE_UUID}" "${APP_PATH}" --json-output target/xcrun.json

# get the installation url by jq-ing the json of the device install
INSTALLATION_URL=$(jq -r '.result.installedApplications[0].installationURL' target/xcrun.json)

export SIMCTL_CHILD_IP_ADDRESS="123123"
export SIMCTL_CHILD_DIOXUS_DEVSERVER_ADDR="ws://127.0.0.1:8080/_dioxus"

# launch the app
# todo: we can just background it immediately and then pick it up for loading its logs
xcrun devicectl device process launch --device "${DEVICE_UUID}" "${INSTALLATION_URL}"

# # launch the app and put it in background
# xcrun devicectl device process launch --no-activate --verbose --device "${DEVICE_UUID}" "${INSTALLATION_URL}" --json-output "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}"

# # Extract background PID of status app
# STATUS_PID=$(jq -r '.result.process.processIdentifier' "${XCRUN_DEVICE_PROCESS_LAUNCH_LOG_DIR}")
# "${GIT_ROOT}/scripts/wait-for-metro-port.sh"  2>&1

# # now that metro is ready, resume the app from background
# xcrun devicectl device process resume --device "${DEVICE_UUID}" --pid "${STATUS_PID}" > "${XCRUN_DEVICE_PROCESS_RESUME_LOG_DIR}" 2>&1






