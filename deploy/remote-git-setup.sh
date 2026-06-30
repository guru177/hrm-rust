#!/usr/bin/env bash
# One-time: clone HRM repo into /opt/hrm (preserves existing deploy/.env).
set -euo pipefail

REPO_URL="${HRM_GIT_REPO:-https://github.com/guru177/hrm-rust.git}"
BRANCH="${HRM_GIT_BRANCH:-main}"
APP_DIR=/opt/hrm

if [ -d "${APP_DIR}/.git" ]; then
  echo "Git repo already initialized at ${APP_DIR}"
  exit 0
fi

echo "==> Install git + Node.js if missing"
if ! command -v git >/dev/null 2>&1; then
  sudo apt-get update -qq
  sudo apt-get install -y -qq git
fi
if ! command -v node >/dev/null 2>&1; then
  curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
  sudo apt-get install -y -qq nodejs
fi
node --version
npm --version

ENV_BACKUP=""
if [ -f "${APP_DIR}/deploy/.env" ]; then
  ENV_BACKUP=$(mktemp)
  cp "${APP_DIR}/deploy/.env" "${ENV_BACKUP}"
  echo "Backed up existing deploy/.env"
fi

if [ -d "${APP_DIR}" ] && [ -n "$(ls -A "${APP_DIR}" 2>/dev/null || true)" ]; then
  BACKUP_DIR="${APP_DIR}.pre-git-$(date +%Y%m%d%H%M%S)"
  echo "==> Moving existing ${APP_DIR} to ${BACKUP_DIR}"
  sudo mv "${APP_DIR}" "${BACKUP_DIR}"
fi

echo "==> Clone ${REPO_URL} (${BRANCH})"
CLONE_TMP=$(mktemp -d)
git clone --branch "${BRANCH}" --depth 1 "${REPO_URL}" "${CLONE_TMP}/hrm"
sudo mkdir -p "${APP_DIR}"
sudo rm -rf "${APP_DIR}"
sudo mv "${CLONE_TMP}/hrm" "${APP_DIR}"
sudo chown -R ubuntu:ubuntu "${APP_DIR}"
rm -rf "${CLONE_TMP}"

if [ -n "${ENV_BACKUP}" ] && [ -f "${ENV_BACKUP}" ]; then
  cp "${ENV_BACKUP}" "${APP_DIR}/deploy/.env"
  rm -f "${ENV_BACKUP}"
  echo "Restored deploy/.env"
elif [ ! -f "${APP_DIR}/deploy/.env" ]; then
  cp "${APP_DIR}/deploy/.env.production.example" "${APP_DIR}/deploy/.env"
  echo "WARN: Created deploy/.env from example — edit secrets before going live"
fi

chmod +x "${APP_DIR}/deploy/"*.sh 2>/dev/null || true
echo "Git setup complete at ${APP_DIR}"
