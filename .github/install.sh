#!/bin/sh
set -eo pipefail

# Reset
Color_Off=''

# Regular Colors
Red=''
Green=''
Dim='' # White

# Bold
Bold_White=''
Bold_Green=''

if [ -t 1 ]; then
    # Reset
    Color_Off='\033[0m' # Text Reset

    # Regular Colors
    Red='\033[0;31m'   # Red
    Green='\033[0;32m' # Green
    Dim='\033[0;2m'    # White

    # Bold
    Bold_Green='\033[1;32m' # Bold Green
    Bold_White='\033[1m'    # Bold White
fi

error() {
    printf "${Red}error${Color_Off}: %s\n" "$*" >&2
    exit 1
}

info() {
    printf "${Dim}%s ${Color_Off}\n" "$*"
}

info_bold() {
    printf "${Bold_White}%s ${Color_Off}\n" "$*"
}

success() {
    printf "${Green}%s ${Color_Off}\n" "$*"
}

command -v unzip >/dev/null ||
    error 'unzip is required to install dx'

if [ $# -gt 2 ]; then
    error 'Too many arguments, only 2 are allowed. The first can be a specific tag of dx to install. (e.g. "dx-v0.7.1") or `nightly` or `pr <PR_NUMBER>` to install the latest nightly or PR build.'
fi

# The gnu builds need glibc 2.35+, so use the static musl build on musl distros (eg Alpine)
# and on systems with an older glibc (eg Amazon Linux 2023)
linux_libc() {
    glibc_version=$(getconf GNU_LIBC_VERSION 2>/dev/null || true)
    case "${glibc_version#glibc }" in
    2.3[5-9]* | 2.[4-9][0-9]* | [3-9].*) echo "gnu" ;;
    *) echo "musl" ;;
    esac
}

if [ "$OS" = "Windows_NT" ]; then
    target="x86_64-pc-windows-msvc"
else
    case $(uname -sm) in
    "Darwin x86_64") target="x86_64-apple-darwin" ;;
    "Darwin arm64") target="aarch64-apple-darwin" ;;
    "Linux aarch64") target="aarch64-unknown-linux-$(linux_libc)" ;;
    *) target="x86_64-unknown-linux-$(linux_libc)" ;;
    esac
fi

GITHUB=${GITHUB-"https://github.com"}
github_repo="$GITHUB/dioxuslabs/dioxus"
exe_name=dx

if [ $# = 0 ]; then
    dx_uri=$github_repo/releases/latest/download/dx-$target.zip
else
    dx_uri=$github_repo/releases/download/$1/dx-$target.zip
fi

if [ -n "$DX_INSTALL" ]; then
    dx_install="$DX_INSTALL"
elif [ -n "$XDG_DATA_HOME" ]; then
    dx_install="$XDG_DATA_HOME/dx"
else
    dx_install="$HOME/.dx"
fi
bin_dir="$dx_install/bin"
exe="$bin_dir/dx"
cargo_bin_dir="${CARGO_HOME:-$HOME/.cargo}/bin"
cargo_bin_exe="$cargo_bin_dir/dx"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
fi

if [ ! -d "$cargo_bin_dir" ]; then
	mkdir -p "$cargo_bin_dir"
fi

curl --fail --location --progress-bar --output "$exe.zip" "$dx_uri"
if command -v unzip >/dev/null; then
	unzip -d "$bin_dir" -o "$exe.zip"
else
	7z x -o"$bin_dir" -y "$exe.zip"
fi
chmod +x "$exe"
cp "$exe" "$cargo_bin_exe" || error "Failed to copy dx to $cargo_bin_dir"
rm "$exe.zip"
echo "  installed: $cargo_bin_exe"

echo
echo "dx was installed successfully! 💫"
echo

if command -v dx >/dev/null; then
	echo "Run 'dx --help' to get started"
else
	echo "Run '$exe --help' to get started"
fi
