#!/usr/bin/env bash
# Build an install package for one target platform.
#
#   usage: install/package.sh <slug> <cargo-target> <kind>
#
#     slug   windows-x64 | linux-x64 | macos-arm64 | macos-x64
#     kind   windows | linux | macos
#
# Expects `cargo build --release --target <cargo-target>` to have run already.
# Writes dist/<slug>/ next to rust/target, containing:
#   windows  <name>-<version>-<slug>.msi   (WiX installer, if WiX is present)
#            <name>-<version>-<slug>.zip   (portable)
#            <name>-<version>-<slug>.exe   (raw binary, always)
#   linux    <name>-<version>-<slug>.tar.gz
#            <name>_<version>_amd64.deb
#   macos    <name>-<version>-<slug>.zip   (StellarDominion.app)
#            <name>-<version>-<slug>.dmg
set -euo pipefail

SLUG="${1:?usage: install/package.sh <slug> <cargo-target> <kind>}"
TARGET="${2:?usage: install/package.sh <slug> <cargo-target> <kind>}"
KIND="${3:?usage: install/package.sh <slug> <cargo-target> <kind>}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${HERE}/.."                                   # rust/

VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
[ -n "${VERSION}" ] || { echo "package.sh: cannot read the crate version" >&2; exit 1; }

NAME="stellar-dominion"
SRC_BIN="target/${TARGET}/release/${NAME}"
[ -f "${SRC_BIN}" ] || {
  echo "package.sh: missing build output ${SRC_BIN}" >&2
  exit 1
}

DIST="$(pwd)/dist/${SLUG}"
STAGE="${DIST}/stage"
rm -rf "${DIST}"
mkdir -p "${STAGE}"

copy_readme() {
  install -m 0644 "${HERE}/README.txt" "${STAGE}/README.txt"
  if [ -f "${HERE}/../../LICENSE" ]; then
    install -m 0644 "${HERE}/../../LICENSE" "${STAGE}/LICENSE.txt"
  fi
}

make_tar_gz() {                                   # make_tar_gz <out> <dir>
  tar -c -z -f "$1" -C "$2" .
}

make_zip() {                                      # make_zip <out> <dir>
  local out="$1" dir="$2"
  if command -v zip >/dev/null 2>&1; then
    ( cd "${dir}" && zip -r -q -X "${out}" . )
  else
    local win_out win_dir
    win_out="$(to_win "${out}")"
    win_dir="$(to_win "${dir}")"
    powershell -NoProfile -Command \
      "Compress-Archive -Path '${win_dir}\*' -DestinationPath '${win_out}' -Force"
  fi
}

# Native Windows path, for tools that cannot read MSYS paths.
to_win() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -m "$1"
  else
    echo "$1"
  fi
}

echo "package.sh: ${NAME} ${VERSION} (${SLUG}, target ${TARGET})"

case "${KIND}" in

  windows)
    install -m 0755 "${SRC_BIN}" "${STAGE}/Stellar Dominion.exe"
    copy_readme
    make_zip "${DIST}/${NAME}-${VERSION}-${SLUG}.zip" "${STAGE}"
    install -m 0755 "${SRC_BIN}" "${DIST}/${NAME}-${VERSION}-${SLUG}.exe"

    WIX_DIR="${WIX_DIR:-}"
    if [ -z "${WIX_DIR}" ]; then
      for d in \
        "/c/Program Files (x86)/WiX Toolset v3.14/bin" \
        "/c/Program Files/WiX Toolset v3.14/bin" \
        "/c/Program Files (x86)/WiX Toolset v3.11/bin" \
        "/c/Program Files/WiX Toolset v3.11/bin"; do
        if [ -x "${d}/candle.exe" ]; then
          WIX_DIR="$d"
          break
        fi
      done
    fi

    if [ -n "${WIX_DIR}" ]; then
      WIN_STAGE="$(to_win "${STAGE}")"
      WIN_DIST="$(to_win "${DIST}")"
      mkdir -p "${DIST}/wix"
      "${WIX_DIR}/candle.exe" -nologo \
        -dVersion="${VERSION}" -dSource="${WIN_STAGE}" \
        -out "$(to_win "${DIST}/wix")/" "${HERE}/stellar-dominion.wxs"
      "${WIX_DIR}/light.exe" -nologo -ext WixUIExtension \
        -out "${WIN_DIST}/${NAME}-${VERSION}-${SLUG}.msi" \
        "$(to_win "${DIST}/wix")/stellar-dominion.wixobj"
      echo "package.sh: wrote ${NAME}-${VERSION}-${SLUG}.msi"
    else
      echo "package.sh: WiX not found - shipping the zip and the raw exe only" >&2
    fi
    ;;

  linux)
    install -m 0755 "${SRC_BIN}" "${STAGE}/${NAME}"
    copy_readme
    make_tar_gz "${DIST}/${NAME}-${VERSION}-${SLUG}.tar.gz" "${STAGE}"

    PKG="${DIST}/deb"
    mkdir -p "${PKG}/DEBIAN" "${PKG}/usr/games" \
      "${PKG}/usr/share/applications" "${PKG}/usr/share/doc/${NAME}"
    sed -e "s/@VERSION@/${VERSION}/" -e "s/@ARCH@/amd64/" \
      "${HERE}/control" > "${PKG}/DEBIAN/control"
    install -m 0755 "${SRC_BIN}" "${PKG}/usr/games/${NAME}"
    install -m 0644 "${HERE}/README.txt" "${PKG}/usr/share/doc/${NAME}/README.txt"
    if [ -f "${HERE}/../../LICENSE" ]; then
      install -m 0644 "${HERE}/../../LICENSE" "${PKG}/usr/share/doc/${NAME}/copyright"
    fi
    install -m 0644 "${HERE}/stellar-dominion.desktop" \
      "${PKG}/usr/share/applications/${NAME}.desktop"
    dpkg-deb --build "${PKG}" "${DIST}/${NAME}_${VERSION}_amd64.deb"
    ;;

  macos)
    APP="${STAGE}/StellarDominion.app"
    mkdir -p "${APP}/Contents/MacOS"
    install -m 0755 "${SRC_BIN}" "${APP}/Contents/MacOS/${NAME}"
    sed -e "s/@VERSION@/${VERSION}/g" "${HERE}/Info.plist" \
      > "${APP}/Contents/Info.plist"
    make_zip "${DIST}/${NAME}-${VERSION}-${SLUG}.zip" "${STAGE}"
    hdiutil create -volname "Stellar Dominion ${VERSION}" \
      -srcfolder "${APP}" -ov -format UDZO \
      -o "${DIST}/${NAME}-${VERSION}-${SLUG}.dmg" >/dev/null 2>&1
    ;;

  *)
    echo "package.sh: unknown kind '${KIND}'" >&2
    exit 1
    ;;
esac

echo "package.sh: artifacts in ${DIST}"
ls -l "${DIST}"
