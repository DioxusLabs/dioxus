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

if [[ -t 1 ]]; then
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
    echo -e "${Red}error${Color_Off}:" "$@" >&2
    exit 1
}

info() {
    echo -e "${Dim}$@ ${Color_Off}"
}

info_bold() {
    echo -e "${Bold_White}$@ ${Color_Off}"
}

success() {
    echo -e "${Green}$@ ${Color_Off}"
}

command -v unzip >/dev/null ||
    error 'unzip is required to install dx'

if [[ $# -gt 1 ]]; then
    error 'Too many arguments, only 1 are allowed. The first can be a specific tag of dx to install. (e.g. "dx-v0.7.1")'
fi

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-pc-windows-msvc"
else
	case $(uname -sm) in
	"Darwin x86_64") target="x86_64-apple-darwin" ;;
	"Darwin arm64") target="aarch64-apple-darwin" ;;
	"Linux aarch64") target="aarch64-unknown-linux-gnu" ;;
	*) target="x86_64-unknown-linux-gnu" ;;
	esac
fi

GITHUB=${GITHUB-"https://github.com"}
github_repo="$GITHUB/dioxuslabs/dioxus"
exe_name=dx

if [[ $# = 0 ]]; then
    dx_uri=$github_repo/releases/latest/download/dx-$target.zip
else
    dx_uri=$github_repo/releases/download/$1/dx-$target.zip
fi

dx_install="${DX_INSTALL:-$HOME/.dx}"
bin_dir="$dx_install/bin"
exe="$bin_dir/dx"
cargo_bin_dir="$HOME/.cargo/bin"
cargo_bin_exe="$cargo_bin_dir/dx"

if [ ! -d "$bin_dir" ]; then
	mkdir -p "$bin_dir"
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
