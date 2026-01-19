# Building sqlkit

This document provides instructions for building the sqlkit application on various platforms.

## Prerequisites

### Rust
Install Rust using rustup:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Node.js and npm
Install Node.js (version 18 or higher) and npm from [nodejs.org](https://nodejs.org/).

## Platform-Specific Dependencies

### Linux (Ubuntu/Debian)

Tauri requires several system libraries. Install them with:

```bash
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libsqlite3-dev \
    pkg-config
```

### Linux (Fedora/RHEL)

```bash
sudo dnf install -y \
    gtk3-devel \
    webkit2gtk4.1-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    openssl-devel \
    sqlite-devel \
    pkg-config
```

### Linux (Arch)

```bash
sudo pacman -S --needed \
    webkit2gtk \
    gtk3 \
    libayatana-appindicator \
    librsvg \
    openssl \
    sqlite \
    pkg-config
```

### macOS

Install Xcode Command Line Tools:
```bash
xcode-select --install
```

### Windows

Install the following:
1. [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed on Windows 11)

## Building

### Development Mode

1. Install npm dependencies:
```bash
npm install
```

2. Run in development mode:
```bash
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

## Testing

### Run all tests:
```bash
cd src-tauri
cargo test
```

### Run specific database adapter tests:

**PostgreSQL:**
```bash
# Set environment variables
export POSTGRES_HOST=localhost
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=your_password
export POSTGRES_DB=testdb

# Run tests
cargo test --test postgres_integration -- --ignored --nocapture
```

**MySQL:**
```bash
# Set environment variables
export MYSQL_HOST=localhost
export MYSQL_PORT=3306
export MYSQL_USER=root
export MYSQL_PASSWORD=your_password
export MYSQL_DB=testdb

# Run tests
cargo test --test mysql_integration -- --ignored --nocapture
```

**SQL Server:**
```bash
# Set environment variables
export SQLSERVER_HOST=localhost
export SQLSERVER_PORT=1433
export SQLSERVER_USER=sa
export SQLSERVER_PASSWORD=YourPassword123!
export SQLSERVER_DB=testdb
export SQLSERVER_TRUST_CERT=true

# Run tests
cargo test --test sqlserver_integration -- --ignored --nocapture
```

## Troubleshooting

### "glib-2.0 not found" error on Linux

This means the GTK development libraries are not installed. Install them using the commands in the Platform-Specific Dependencies section above.

### "webkit2gtk not found" error on Linux

Make sure you have `libwebkit2gtk-4.1-dev` (Ubuntu 24.04+) or `libwebkit2gtk-4.0-dev` (older versions) installed.

### Build fails on macOS

Ensure Xcode Command Line Tools are installed:
```bash
xcode-select --install
```

### Build fails on Windows

1. Install Microsoft C++ Build Tools
2. Ensure WebView2 is installed
3. Restart your terminal/IDE after installing dependencies

## CI/CD

The project uses GitHub Actions for continuous integration. See `.github/workflows/` for workflow configurations.

## Additional Resources

- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
- [Rust Installation](https://www.rust-lang.org/tools/install)
- [Node.js Downloads](https://nodejs.org/)
