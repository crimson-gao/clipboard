#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="$(node -p "require('${ROOT_DIR}/src-tauri/tauri.conf.json').productName")"
APP_VERSION="$(node -p "require('${ROOT_DIR}/src-tauri/tauri.conf.json').version")"
ARCH="$(uname -m)"
if [[ "${ARCH}" == "arm64" ]]; then
  ARCH="aarch64"
fi

APP_BUNDLE_PATH="${ROOT_DIR}/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
DMG_OUTPUT_PATH="${ROOT_DIR}/src-tauri/target/release/bundle/dmg/${APP_NAME}_${APP_VERSION}_${ARCH}.dmg"
TEMP_DMG_PATH="/tmp/${APP_NAME}_${APP_VERSION}_${ARCH}.dmg"
STAGE_DIR="/tmp/${APP_NAME}-dmg-stage"

if [[ ! -d "${APP_BUNDLE_PATH}" ]]; then
  echo "App bundle not found: ${APP_BUNDLE_PATH}" >&2
  echo "Run 'npm run build:app' first." >&2
  exit 1
fi

rm -f "${TEMP_DMG_PATH}"
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}"

cp -R "${APP_BUNDLE_PATH}" "${STAGE_DIR}/${APP_NAME}.app"
ln -s /Applications "${STAGE_DIR}/Applications"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${STAGE_DIR}" \
  -ov \
  -format UDZO \
  "${TEMP_DMG_PATH}"

mkdir -p "$(dirname "${DMG_OUTPUT_PATH}")"
mv -f "${TEMP_DMG_PATH}" "${DMG_OUTPUT_PATH}"

echo "DMG created at: ${DMG_OUTPUT_PATH}"
