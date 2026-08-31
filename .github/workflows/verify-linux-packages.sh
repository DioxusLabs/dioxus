#!/usr/bin/env bash
# Inspect Linux bundle artifacts produced by `dx bundle`.
#
# Each packager's dependency field must equal its config list (RPM also has
# rpmlib(...) markers from the format). Isolation follows: extra names from
# the other format fail the equality check, shared names are allowed.
set -euo pipefail

target_dir=${TARGET_DIR:-target}
deb_depends=${DEB_DEPENDS:-libwebkit2gtk-4.1-0}
rpm_requires=${RPM_REQUIRES:-webkit2gtk4.1}
desktop_template_test_id=${DESKTOP_TEMPLATE_TEST_ID:-linux-desktop-template-test-id}

assert_equal_deps() {
  local label=$1 configured=$2 packaged=$3
  if [[ "$configured" != "$packaged" ]]; then
    echo "${label} mismatch" >&2
    echo "configured: ${configured}" >&2
    echo "packaged: ${packaged}" >&2
    exit 1
  fi
}

mapfile -t appimages < <(find "$target_dir" -name '*.AppImage' -print)
mapfile -t debs < <(find "$target_dir" -name '*.deb' -print)
mapfile -t rpms < <(find "$target_dir" -name '*.rpm' -print)

printf '%s\n' "${appimages[@]+"${appimages[@]}"}" "${debs[@]+"${debs[@]}"}" "${rpms[@]+"${rpms[@]}"}"

if (( ${#appimages[@]} != 1 || ${#debs[@]} != 1 || ${#rpms[@]} != 1 )); then
  echo "expected one AppImage, one .deb, and one .rpm under ${target_dir}" >&2
  exit 1
fi

deb=${debs[0]}
rpm_pkg=${rpms[0]}

packaged_deb_depends=$(dpkg-deb -f "$deb" Depends)
echo "Depends: ${packaged_deb_depends}"
assert_equal_deps "deb Depends" "$deb_depends" "$packaged_deb_depends"

packaged_rpm_requires=$(rpm -qp --requires "$rpm_pkg")
echo "$packaged_rpm_requires"
assert_equal_deps "rpm Requires" "$rpm_requires" \
  "$(printf '%s\n' "$packaged_rpm_requires" | awk '!/^rpmlib\(/')"

staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
mkdir -p "$staging/deb" "$staging/rpm"
dpkg-deb -x "$deb" "$staging/deb"
rpm2cpio "$rpm_pkg" | (cd "$staging/rpm" && cpio -idm --quiet --no-absolute-filenames)
grep -F -q "$desktop_template_test_id" "$staging/deb"/usr/share/applications/*.desktop
grep -F -q "$desktop_template_test_id" "$staging/rpm"/usr/share/applications/*.desktop
