# SqlKit

[![CI](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml/badge.svg)](https://github.com/geek-fun/sqlkit/actions/workflows/node.yml)
[![Release](https://github.com/geek-fun/sqlkit/actions/workflows/release.yml/badge.svg)](https://github.com/geek-fun/sqlkit/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

AI-powered cross-platform SQL database GUI client, supporting PostgreSQL, MySQL, Oracle, SQL Server, DB2, SQLite, H2, and ClickHouse on macOS, Windows, and Linux.

## Features

- 🗄️ **Multi-database support** - PostgreSQL, MySQL, Oracle, SQL Server, DB2, SQLite, H2, ClickHouse
- 🤖 **AI-powered** - Integrated AI assistant for query generation and optimization
- 🖥️ **Cross-platform** - Native apps for macOS, Windows, and Linux
- 🔒 **Secure** - Encrypted credential storage, SSH tunnel support
- 📊 **Data Studio** - Visualize and explore your data
- 🔄 **Import/Export** - Transfer data between databases

## Download

Download the latest release from [GitHub Releases](https://github.com/geek-fun/sqlkit/releases).

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `SqlKit_*_aarch64.dmg` |
| macOS (Intel) | `SqlKit_*_x64.dmg` |
| Windows | `SqlKit_*_x64-setup.exe` |
| Linux | `SqlKit_*_amd64.AppImage` or `.deb` |

## Development

### Prerequisites

Before building, ensure you have the required system dependencies installed. See [BUILD.md](BUILD.md) for detailed instructions.

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

**macOS:**

```bash
xcode-select --install
```

**Windows:** Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

### Quick Start

```bash
npm install
npm run tauri dev
```

### Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start development server |
| `npm run build` | Build for production |
| `npm run lint:check` | Run ESLint |
| `npm run test` | Run tests |
| `npm run tauri dev` | Start Tauri development |
| `npm run tauri build` | Build Tauri application |

## Documentation

- [BUILD.md](BUILD.md) - Build instructions and troubleshooting
- [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) - Distribution and code signing

## CI/CD

SqlKit uses GitHub Actions for continuous integration and deployment:

- **CI Pipeline** (`node.yml`) - Runs on PRs to master
  - Lint, test, and build across macOS, Windows, Linux

- **Release Pipeline** (`release.yml`) - Runs on push to master
  - Builds platform-specific installers
  - Creates GitHub release
  - Supports code signing and notarization

See [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) for configuration details.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Tech Stack

- **Frontend:** Vue 3, TypeScript, Vite
- **Backend:** Rust, Tauri v2
- **Database Drivers:** sqlx, tauri-plugin-sql
- **UI Components:** shadcn-vue, Tailwind CSS

## License

[MIT](LICENSE)

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a PR.
