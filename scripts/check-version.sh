#!/bin/bash -eu
set -o pipefail

# Usage: check-version.sh [package-path] [tag-prefix]
# Example: check-version.sh . v

PACKAGE_PATH="${1:-.}"
TAG_PREFIX="${2:-v}"

# Navigate to repository root
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit

# Read package version
PACKAGE_VERSION=$(node -p "require('${PACKAGE_PATH}/package.json').version")
TAG_NAME="${TAG_PREFIX}${PACKAGE_VERSION}"

echo "version=${PACKAGE_VERSION}"
echo "tag_name=${TAG_NAME}"

# Check if tag already exists
if git rev-parse "$TAG_NAME" >/dev/null 2>&1; then
  echo "Tag ${TAG_NAME} already exists, skipping release"
  echo "should_release=false"
  
  # Output for GitHub Actions
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "version=${PACKAGE_VERSION}" >> "$GITHUB_OUTPUT"
    echo "tag_name=${TAG_NAME}" >> "$GITHUB_OUTPUT"
    echo "should_release=false" >> "$GITHUB_OUTPUT"
  fi
  
  exit 0
else
  echo "New version detected: ${PACKAGE_VERSION}"
  echo "should_release=true"
  
  # Output for GitHub Actions
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "version=${PACKAGE_VERSION}" >> "$GITHUB_OUTPUT"
    echo "tag_name=${TAG_NAME}" >> "$GITHUB_OUTPUT"
    echo "should_release=true" >> "$GITHUB_OUTPUT"
  fi
  
  exit 0
fi