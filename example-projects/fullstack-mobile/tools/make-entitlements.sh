# get the mobile provisioning profile
export APP_DEV_NAME=$(xcrun security find-identity -v -p codesigning | grep "Apple Development: " | sed -E 's/.*"([^"]+)".*/\1/')

# Find the provisioning profile from ~/Library/MobileDevice/Provisioning\ Profiles
export PROVISION_FILE=$(ls ~/Library/MobileDevice/Provisioning\ Profiles | grep mobileprovision)

# Convert the provisioning profile to json so we can use jq to extract the important bits
security cms -D \
	-i ~/Library/MobileDevice/Provisioning\ Profiles/${PROVISION_FILE} | \
	python3 -c 'import plistlib,sys,json; print(json.dumps(plistlib.loads(sys.stdin.read().encode("utf-8")), default=lambda o:"<not serializable>"))' \
	> target/provisioning.json

# jq out the important bits of the provisioning profile
export TEAM_IDENTIFIER=$(jq -r '.TeamIdentifier[0]' target/provisioning.json)
export APPLICATION_IDENTIFIER_PREFIX=$(jq -r '.ApplicationIdentifierPrefix[0]' target/provisioning.json)
export APPLICATION_IDENTIFIER=$(jq -r '.Entitlements."application-identifier"' target/provisioning.json)
export APP_ID_ACCESS_GROUP=$(jq -r '.Entitlements."keychain-access-groups"[0]' target/provisioning.json)

# now build the entitlements file
cat <<EOF > target/entitlements.xcent
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
		<key>application-identifier</key>
		<string>${APPLICATION_IDENTIFIER}</string>
		<key>keychain-access-groups</key>
		<array>
			<string>${APP_ID_ACCESS_GROUP}.*</string>
		</array>
		<key>get-task-allow</key>
		<true/>
		<key>com.apple.developer.team-identifier</key>
		<string>${TEAM_IDENTIFIER}</string>
</dict></plist>
EOF

# sign the app
codesign --force \
	--entitlements target/entitlements.xcent \
	--sign "${APP_DEV_NAME}" \
	target/aarch64-apple-ios/debug/bundle/ios/DioxusApp.app
