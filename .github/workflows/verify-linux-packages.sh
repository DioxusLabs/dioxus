#!/usr/bin/env bash
# Inspect the package metadata and payload produced by the Linux bundle smoke test.
set -euo pipefail

target_dir=${TARGET_DIR:-target}

mapfile -t appimages < <(find "$target_dir" -name '*.AppImage' -print)
mapfile -t debs < <(find "$target_dir" -name '*.deb' -print)
mapfile -t rpms < <(find "$target_dir" -name '*.rpm' -print)

printf '%s\n' "${appimages[@]+"${appimages[@]}"}" "${debs[@]+"${debs[@]}"}" "${rpms[@]+"${rpms[@]}"}"

if (( ${#appimages[@]} != 1 || ${#debs[@]} != 1 || ${#rpms[@]} != 1 )); then
  echo "expected one AppImage, one .deb, and one .rpm under ${target_dir}" >&2
  exit 1
fi

appimage=$(readlink -f "${appimages[0]}")
deb=${debs[0]}
rpm_pkg=${rpms[0]}

packaged_deb_depends=$(dpkg-deb -f "$deb" Depends)
if [[ "$packaged_deb_depends" != "$DEB_DEPENDS" ]]; then
  echo "deb Depends mismatch: expected '$DEB_DEPENDS', got '$packaged_deb_depends'" >&2
  exit 1
fi

packaged_rpm_requires=$(rpm -qp --requires "$rpm_pkg" | awk '!/^rpmlib\(/')
if [[ "$packaged_rpm_requires" != "$RPM_REQUIRES" ]]; then
  echo "rpm Requires mismatch: expected '$RPM_REQUIRES', got '$packaged_rpm_requires'" >&2
  exit 1
fi

rpm -qp --recommends "$rpm_pkg" | grep -Fqx "$RPM_RECOMMENDS"
rpm -qp --provides "$rpm_pkg" | grep -Fqx "$RPM_PROVIDES"
rpm -qp --conflicts "$rpm_pkg" | grep -Fqx "$RPM_CONFLICTS"
rpm -qp --obsoletes "$rpm_pkg" | grep -Fqx "$RPM_OBSOLETES"

staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT
mkdir -p "$staging/deb" "$staging/rpm" "$staging/appimage"

dpkg-deb -x "$deb" "$staging/deb"
rpm2cpio "$rpm_pkg" | (cd "$staging/rpm" && cpio -idm --quiet --no-absolute-filenames)
(cd "$staging/appimage" && "$appimage" --appimage-extract)

grep -Fq "$DESKTOP_TEMPLATE_TEST_ID" "$staging/deb"/usr/share/applications/*.desktop
grep -Fq "$DESKTOP_TEMPLATE_TEST_ID" "$staging/rpm"/usr/share/applications/*.desktop
grep -Fq "$DESKTOP_TEMPLATE_TEST_ID" "$staging/appimage"/squashfs-root/usr/share/applications/*.desktop

# Prove that [linux.rpm.files] copied the named test fixture to its configured
# package path without silently omitting it.
grep -Fqx "$RPM_FILE_TEST_TOKEN" "$staging/rpm/usr/share/hotdog/rpm-file-test.txt"
