# Baccy

A cross-platform BACnet browser and testing tool built with Tauri, Rust, and Svelte.

## Features

### Transport
- **BACnet/IP** — UDP-based communication on standard BACnet ports (47808-47823)
- **MS/TP** — Full Master-Slave/Token-Passing over RS-485 serial (9600-115200 baud)
- **BBMD** — BACnet Broadcast Management Device (foreign device registration, forwarding)
- **BBMD UI** — Right-panel tab for BBMD control: enable/disable, foreign device registration with IP/TTL, FDT viewer with entry management
- **BACnet Router** — Multi-interface routing with DNET/DADR/SLR support
- **Router UI** — Right-panel tab for route management: view routing table, interfaces, add/remove routes
- **Packet Inspector** — Live hex dump of all BACnet traffic with TX/RX coloring, pause/resume, download
- **Network Stats** — Real-time metrics: packets sent/received, bytes, errors, avg response time

### Device Discovery
- **Who-Is/I-Am** — Discover all BACnet devices on the network
- **Who-Is Range** — Scan specific device instance ranges (e.g., 1000-2000)
- **Who-Has/I-Have** — Find objects by name or object identifier across the network
- **Device Health** — Online/offline tracking with consecutive failure detection
- **Device Info** — Rich device details: vendor, model, firmware, protocol version, APDU config

### Object Browsing
- **40+ Object Types** — All standard BACnet types (AI, AO, AV, BI, BO, BV, MSI, MSO, MSV, Schedule, Calendar, TrendLog, LifeSafety, etc.)
- **Object List** — Grouped by type, sortable, searchable
- **104 Property IDs** — All standard properties with human-readable names
- **Dynamic Property Discovery** — Reads PropertyList for automatic property enumeration
- **Write Protection** — Per-device/object/property rules with confirmation dialogs
- **Schedule/Calendar Viewer** — Specialized view for Schedule/Calendar objects showing PresentValue, DescriptionOfSchedule, and DateList

### Read/Write
- **ReadProperty** — Read any individual property
- **ReadPropertyMultiple** — Batch-read multiple properties in one request
- **WriteProperty** — Write with priority array support (1-16)
- **WritePropertyMultiple** — Atomic write to multiple objects
- **Property Value Parsing** — Real, Integer, Unsigned, Boolean, String, Enumerated, BitString, ObjectIdentifier
- **AtomicReadFile** — Read file objects (binary data as hex); key for firmware/trendlog download
- **AtomicWriteFile** — Write binary data to file objects
- **UnconfirmedPrivateTransfer** — Vendor-specific broadcast messaging (vendor ID + service number + optional data)

### Change of Value (COV)
- **SubscribeCOV** — Subscribe to object-level change notifications
- **SubscribeCOVProperty** — Subscribe to specific property changes with COV increment
- **COV Notification Routing** — Decodes and routes unconfirmed COV notifications to subscribers

### Trending & Charts
- **Real-time Trending** — Poll-based data collection with configurable interval (1-60s)
- **Chart.js Integration** — Interactive time-series charts with zoom, pan, tooltips
- **Multi-Property Overlay** — Trend multiple properties simultaneously with color coding
- **Visibility Toggle** — Show/hide individual trend lines
- **CSV Export** — Export trend data to CSV files
- **Parquet Export** — Export trend data to Apache Parquet format

### Alarming & Events
- **Event Notification Parsing** — Decodes unconfirmed event notifications (initiating device, event object, timestamp, priority, notify type)
- **ConfirmedEventNotification** — Request event notification data from devices
- **AcknowledgeAlarm** — Acknowledge alarms with state tracking
- **DeviceCommunicationControl** — Enable/disable device communication (with password)
- **ReinitializeDevice** — Coldstart/warmstart/backup/restore operations

### Time Services
- **TimeSynchronization** — Send local time to individual devices
- **UTCTimeSynchronization** — Global broadcast of UTC time

### Comparison
- **Side-by-Side Object Comparison** — Compare properties across 2+ objects from any devices
- **Diff Highlighting** — Automatically highlights differing values with color-coded cells
- **Show Differences Only** — Filter to rows with differences

### Configuration
- **Export Device Config** — Dump all objects and properties to TOML
- **Import Device Config** — Load and apply TOML configuration
- **Resizable Layout** — Three-pane layout (device tree, object/properties, trending) with persistence
- **Keyboard Shortcuts** — Ctrl+D (discover), Ctrl+R (refresh), Ctrl+, (preferences), Ctrl+P (packet inspector)
- **Preferences** — Auto-refresh, confirm property write, status bar toggle, trending interval

### Platform
- **Cross-Platform** — Windows, macOS, Linux (Tauri v2)
- **Serial Port Support** — Automatic serial port enumeration with USB/PCI/Bluetooth detection
- **Network Interface Selection** — Choose specific NIC or use all interfaces
- **Error Handling** — User-friendly error messages with actionable hints (permission denied, port in use, etc.)

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
