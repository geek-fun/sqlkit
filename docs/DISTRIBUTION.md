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
- Creates GitHub release automatically
- Uploads all artifacts (DMG, EXE, AppImage, DEB)
- Generates updater manifests (`latest.json`)

---

## How It Works

The simplified workflow uses `tauri-action` which handles everything automatically:

```yaml
- uses: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
    APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_ID_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
  with:
    tagName: v__VERSION__
    releaseName: SqlKit v__VERSION__
    releaseBody: See the assets to download this version and install.
    releaseDraft: true
    prerelease: false
```

**tauri-action automatically:**
- ✅ Creates GitHub release
- ✅ Collects all artifacts (DMG, EXE, AppImage, DEB)
- ✅ Generates `latest.json` for updater
- ✅ Creates `.sig` files for signing
- ✅ Imports certificates and handles keychain (macOS)
- ✅ Notarizes with Apple (macOS)

---

## macOS Code Signing Options

### Option 1: No Signing (Development)

Skip signing entirely for early development.

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

**Result:** Users see "unverified developer" warning.

---

### Option 2: Full Code Signing (Production)

Set these GitHub secrets for full signing and notarization:

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` certificate file |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `APPLE_SIGNING_IDENTITY` | Certificate identity (e.g., `Developer ID Application: Your Name (TEAM_ID)`) |
| `APPLE_ID` | Your Apple ID email address |
| `APPLE_ID_PASSWORD` | App-specific password from Apple ID |
| `APPLE_TEAM_ID` | Your Apple Team ID |

**Result:** Fully signed and notarized app, no warnings.

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

### 6. Get Signing Identity

Run this command to get your signing identity:

```bash
security find-identity -v -p codesigning
```

The output looks like: `Developer ID Application: Your Name (TEAM_ID)`

Add this to `APPLE_SIGNING_IDENTITY` secret.

---

## Windows Code Signing (Optional)

Windows code signing requires Azure Key Vault:

| Secret | Description |
|--------|-------------|
| `AZURE_CLIENT_ID` | Azure client ID |
| `AZURE_CLIENT_SECRET` | Azure client secret |
| `AZURE_TENANT_ID` | Azure tenant ID |
| `AZURE_KEY_VAULT_URL` | Azure Key Vault URL |
| `AZURE_KEY_VAULT_CERTIFICATE` | Certificate name in Key Vault |

---

## Linux Distribution

No code signing required. The workflow generates:

- **AppImage**: `SqlKit_VERSION_amd64.AppImage`
- **DEB**: `sql-kit_VERSION_amd64.deb`

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

**Required GitHub Secrets:**

| Secret | Description |
|--------|-------------|
| `TAURI_SIGNING_PRIVATE_KEY` | Private key for signing updates |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the private key |

### Generate Updater Keys

```bash
npm run tauri signer generate -- -w ~/.tauri/sqlkit.key
```

---

## Artifact Names

Built artifacts follow Tauri conventions:

| Platform | Artifact Name |
|----------|---------------|
| macOS DMG | `SqlKit_VERSION_aarch64.dmg`, `SqlKit_VERSION_x64.dmg` |
| macOS App | `SqlKit_aarch64.app.tar.gz`, `SqlKit_x64.app.tar.gz` |
| Windows | `SqlKit_VERSION_x64-setup.exe` |
| Linux AppImage | `SqlKit_VERSION_amd64.AppImage` |
| Linux DEB | `sql-kit_VERSION_amd64.deb` |
| Updater Manifest | `latest.json` |

---

## References

- [Tauri Distribution Guide](https://v2.tauri.app/distribute/)
- [Tauri GitHub Actions Guide](https://v2.tauri.app/distribute/pipelines/github/)
- [macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/)
- [Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/)
