#!/bin/bash
echo "Update Apt Database"
sudo apt-get update

echo "Install Webkit Dependencies"
sudo apt-get -qq install build-essential libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev
