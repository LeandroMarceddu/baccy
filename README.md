# Baccy

A cross-platform BACnet browser and testing tool built with Tauri, Rust, and Svelte.

## Features

- Device discovery via Who-Is/I-Am broadcasts
- Browse BACnet objects and properties
- Read and write property values
- Real-time trending with historical charts
- Network interface selection
- Cross-platform support (Windows, macOS, Linux)

## Requirements

- Node.js 18+ and npm
- Rust 1.70+
- System dependencies for Tauri (see [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites))

## Development Build

1. Clone the repository:
```bash
git clone https://github.com/LeandroMarceddu/baccy.git
cd baccy
```

2. Install dependencies:
```bash
npm install
```

3. Run in development mode:
```bash
npm run tauri dev
```

## Production Build

Build for your current platform:

```bash
npm run tauri build
```

The compiled application will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
baccy/
├── src/                    # Svelte frontend
├── src-tauri/              # Tauri backend
├── crates/                 # Rust crates
│   ├── baccy-core/         # Core types
│   ├── baccy-transport/    # Network transport
│   ├── baccy-protocol/     # BACnet protocol
│   └── baccy-app/          # Application logic
└── public/                 # Static assets
```

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome. Please open an issue or pull request on GitHub.
