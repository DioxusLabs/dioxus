#!/bin/bash
# Setup Android environment for Dioxus mobile development

export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/27.0.12077973

# Add to PATH if not already there
export PATH=$PATH:$ANDROID_HOME/platform-tools
export PATH=$PATH:$ANDROID_HOME/tools
export PATH=$PATH:$ANDROID_HOME/cmdline-tools/latest/bin

echo "✅ Android environment configured!"
echo "ANDROID_HOME: $ANDROID_HOME"
echo "ANDROID_NDK_HOME: $ANDROID_NDK_HOME"
echo ""
echo "Now you can run:"
echo "  dx serve --android"
echo ""

