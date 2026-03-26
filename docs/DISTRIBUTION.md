# Distribution & Code Signing

This document describes the distribution and code signing setup for SQLKit across macOS, Windows, and Linux platforms.

## Overview

SQLKit uses GitHub Actions with `tauri-apps/tauri-action` for building and releasing cross-platform binaries:

- **macOS**: DMG installer with optional code signing and notarization
- **Windows**: NSIS installer with optional code signing
- **Linux**: AppImage and DEB packages

## CI/CD Workflows

### `node.yml` - CI Pipeline

Runs on pull requests to `master` branch:
- Lint and test across macOS, Windows, Linux
- Build verification (no release artifacts)

### `release.yml` - Release Pipeline

Runs on push to `master` branch:
- Builds platform-specific installers
- Creates GitHub release with auto-generated changelog
- Uploads artifacts to GitHub Releases

---

## macOS Code Signing Options

Code signing on macOS is required to prevent "unverified developer" warnings. There are three approaches:

### Option 1: Ad-Hoc Signing (Simplest)

No Apple Developer certificate required. Good for early development and testing.

**Configuration in `tauri.conf.json`:**

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "-"
    }
  }
}
```

**Pros:**
- No secrets management
- No Apple Developer account required
- Simplest setup

**Cons:**
- Users see "unverified developer" warning
- Users must whitelist app in Privacy & Security settings
- Not suitable for production distribution

**Required GitHub Secrets:** None

---

### Option 2: Dedicated GitHub Action (Recommended)

Uses `apple-actions/import-codesign-certs` for cleaner certificate management.

**Workflow configuration:**

```yaml
- name: Import Apple Certificate
  if: matrix.os == 'macos-latest'
  uses: apple-actions/import-codesign-certs@v3
  with:
    p12-file-base64: ${{ secrets.APPLE_CERTIFICATE }}
    p12-password: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}

- uses: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_ID_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

**Required GitHub Secrets:**

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` certificate file |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `APPLE_ID` | Your Apple ID email address |
| `APPLE_ID_PASSWORD` | App-specific password from Apple ID |
| `APPLE_TEAM_ID` | Your Apple Team ID |

**Pros:**
- Cleaner than manual script
- Same security as full approach
- Automatically handles keychain setup

**Cons:**
- Requires paid Apple Developer account ($99/year)
- Requires certificate setup

---

### Option 3: Manual Certificate Import (Current)

Uses shell commands to import certificate into a temporary keychain.

**Workflow configuration:**

```yaml
- name: Import Apple Developer Certificate
  if: matrix.os == 'macos-latest'
  env:
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    KEYCHAIN_PASSWORD: ${{ secrets.KEYCHAIN_PASSWORD }}
  run: |
    echo $APPLE_CERTIFICATE | base64 --decode > certificate.p12
    security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security default-keychain -s build.keychain
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
    security set-keychain-settings -t 3600 -u build.keychain
    security import certificate.p12 -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" build.keychain
    security find-identity -v -p codesigning build.keychain

- name: Verify Certificate
  if: matrix.os == 'macos-latest'
  run: |
    CERT_INFO=$(security find-identity -v -p codesigning build.keychain | grep "Developer ID Application")
    CERT_ID=$(echo "$CERT_INFO" | awk -F'"' '{print $2}')
    echo "CERT_ID=$CERT_ID" >> $GITHUB_ENV

- uses: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_ID_PASSWORD }}
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    APPLE_SIGNING_IDENTITY: ${{ env.CERT_ID }}
```

**Required GitHub Secrets:**

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` certificate file |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `KEYCHAIN_PASSWORD` | Password for temporary keychain |
| `APPLE_ID` | Your Apple ID email address |
| `APPLE_ID_PASSWORD` | App-specific password from Apple ID |
| `APPLE_TEAM_ID` | Your Apple Team ID |

---

## Setting Up Apple Developer Certificate

### 1. Create Certificate Signing Request (CSR)

On a Mac, open **Keychain Access** → **Certificate Assistant** → **Request a Certificate From a Certificate Authority**.

### 2. Create Certificate

1. Go to [Apple Developer Certificates](https://developer.apple.com/account/resources/certificates/list)
2. Click **Create a certificate**
3. Choose **Developer ID Application** (for distribution outside App Store)
4. Upload your CSR file

### 3. Export Certificate

1. Download the `.cer` file and double-click to install
2. Open **Keychain Access** → **My Certificates**
3. Right-click the certificate → **Export**
4. Save as `.p12` with a password

### 4. Convert to Base64

```bash
openssl base64 -A -in certificate.p12 -out certificate-base64.txt
```

### 5. Add to GitHub Secrets

Copy the contents of `certificate-base64.txt` to `APPLE_CERTIFICATE` secret.

---

## Windows Code Signing (Optional)

Windows code signing requires a code signing certificate from a trusted CA.

**Required GitHub Secrets:**

| Secret | Description |
|--------|-------------|
| `WINDOWS_CERTIFICATE` | Base64-encoded certificate file |
| `WINDOWS_CERTIFICATE_PASSWORD` | Certificate password |

The `tauri-action` will automatically use these for Windows builds.

---

## Linux Distribution

No code signing required for Linux. The workflow generates:

- **AppImage**: Portable executable
- **DEB**: Debian/Ubuntu package

---

## Tauri Updater Configuration

For auto-updates, configure in `tauri.conf.json`:

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "YOUR_PUBLIC_KEY",
      "endpoints": [
        "https://github.com/geek-fun/sqlkit/releases/latest/download/latest.json"
      ]
    }
  }
}
```

**Required GitHub Secrets for Updater:**

| Secret | Description |
|--------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key for signing updates |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key |

### Generate Updater Keys

```bash
npm run tauri signer generate -- -w ~/.tauri/sqlkit.key
```

---

## Current Implementation

SQLKit currently uses **Option 3 (Manual Certificate Import)** with full code signing and notarization support.

To switch to a simpler approach:
1. For development: Use Option 1 (Ad-Hoc)
2. For production: Use Option 2 (Dedicated Action)

---

## References

- [Tauri Distribution Guide](https://v2.tauri.app/distribute/)
- [Tauri GitHub Actions Guide](https://v2.tauri.app/distribute/pipelines/github/)
- [macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/)
- [Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/)
