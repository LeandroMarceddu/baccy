# Baccy

A cross-platform BACnet browser and testing tool built with Tauri, Rust, and Svelte.

## Changelog

### 2026-04-19 - MS/TP Support & Improvements
- **Added MS/TP transport support** - Full BACnet MS/TP (Master-Slave/Token-Passing) implementation for RS-485 serial networks
- **Upgraded bacnet-rs** - Updated from v0.2.2 to v0.3 with improved type safety
- **Serial port configuration** - Support for baud rates: 9600, 19200, 38400, 76800, 115200
- **Token passing state machine** - Proper MS/TP master node token management
- **Transport selection UI** - Switch between BACnet/IP and MS/TP in network setup dialog
- **Improved logging** - Cleaner discovery logs without timeout noise
- **Bug fixes** - Fixed token state initialization for immediate frame transmission

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
## AI Used? 

Yes.

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome. Please open an issue or pull request on GitHub.
