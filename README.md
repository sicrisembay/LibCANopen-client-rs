# libCANopen Client (Rust)

A complete CANopen master/client implementation in Rust with async/await support and PEAK CAN hardware integration.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/sicrisembay/LibCANopen-client-rs/workflows/CI/badge.svg)](https://github.com/sicrisembay/LibCANopen-client-rs/actions)
[![Tests](https://img.shields.io/badge/tests-161%20passing-brightgreen.svg)](https://github.com/sicrisembay/LibCANopen-client-rs)

## Features

### ✅ Complete Protocol Implementation

- **SDO (Service Data Objects)** - Expedited and segmented transfers for reading/writing object dictionary entries
- **NMT (Network Management)** - Node state control, heartbeat monitoring, and state transitions
- **PDO (Process Data Objects)** - High-speed real-time data exchange with mapping support
- **SYNC (Synchronization)** - Network synchronization with counter support (up to 100 Hz tested)
- **EMCY (Emergency)** - Emergency message handling with 40+ standard error codes
- **LSS (Layer Setting Services)** - Node configuration and commissioning (all 14 commands)

### 🚀 Modern Rust Architecture

- **Async/await** based API using Tokio runtime
- **Type-safe** message handling with compile-time checks
- **Thread-safe** concurrent access with Arc + RwLock
- **Zero-copy** where possible for performance
- **Event-driven** callbacks for real-time notifications
- **Comprehensive error handling** with descriptive error types

### 🔌 Hardware Support

- **PEAK CAN USB** adapters (via peak-can-sys)
- Support for 10 kbps to 1 Mbps bus speeds
- Tested at 1 Mbps with 50+ PDO messages/second

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
libcanopen-client = "0.1"
tokio = { version = "1.0", features = ["full"] }
env_logger = "0.11"  # Optional: for logging
```

### Basic Usage

```rust
use libcanopen_client::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Create and connect
    let mut canopen = CANopenSimple::new();
    canopen.connect(BusSpeed::Baud1M).await?;

    // Read device type from node 5
    let device_type = canopen.sdo_read_u32(5, 0x1000, 0).await?;
    println!("Device type: 0x{:08X}", device_type);

    // Write heartbeat producer time (1000ms)
    canopen.sdo_write_u16(5, 0x1017, 0, 1000).await?;

    // Start node in operational mode
    canopen.nmt_start(5).await?;

    // Register PDO callback
    canopen.register_pdo_callback(0x185, |data| {
        println!("PDO received: {:02X?}", data);
    }).await;

    // Send SYNC message
    canopen.send_sync().await?;

    Ok(())
}
```

## Protocol Examples

### SDO (Service Data Objects)

Read and write to device object dictionary:

```rust
// Read operations
let value_u8 = canopen.sdo_read_u8(node_id, 0x1001, 0).await?;
let value_u16 = canopen.sdo_read_u16(node_id, 0x1017, 0).await?;
let value_u32 = canopen.sdo_read_u32(node_id, 0x1000, 0).await?;
let raw_data = canopen.sdo_read(node_id, 0x1018, 1).await?;

// Write operations  
canopen.sdo_write_u8(node_id, 0x1001, 0, 0x00).await?;
canopen.sdo_write_u16(node_id, 0x1017, 0, 1000).await?;
canopen.sdo_write_u32(node_id, 0x1000, 0, 0x12345678).await?;
canopen.sdo_write(node_id, 0x1018, 1, vec![1,2,3,4]).await?;
```

**Segmented transfers** (>4 bytes) are handled automatically!

### NMT (Network Management)

Control node states and monitor heartbeats:

```rust
// Node control
canopen.nmt_start(node_id).await?;              // Enter operational
canopen.nmt_stop(node_id).await?;               // Enter stopped
canopen.nmt_enter_pre_operational(node_id).await?;  // Pre-operational
canopen.nmt_reset_node(node_id).await?;         // Reset application
canopen.nmt_reset_communication(node_id).await?; // Reset communication

// State monitoring
let state = canopen.get_node_state(node_id).await?;
println!("Node state: {:?}", state);

// Heartbeat callback
canopen.register_heartbeat_callback(node_id, |node_id, state| {
    println!("Node {} state changed to {:?}", node_id, state);
}).await;
```

### PDO (Process Data Objects)

High-speed real-time data exchange:

```rust
// Send PDO
canopen.send_pdo(0x185, vec![0x01, 0x02, 0x03, 0x04]).await?;

// Register callback for received PDOs
canopen.register_pdo_callback(0x285, |data| {
    println!("Received PDO: {:02X?}", data);
}).await;

// Unregister callback
canopen.unregister_pdo_callback(0x285).await;
```

### SYNC (Synchronization)

Network synchronization for coordinated PDO transmission:

```rust
// Simple SYNC
canopen.send_sync().await?;

// SYNC with counter (1-240)
canopen.set_sync_counter_enabled(true).await;
canopen.send_sync().await?;  // Counter increments automatically

// Monitor SYNC reception
canopen.register_sync_callback(|counter| {
    println!("SYNC received, counter: {:?}", counter);
}).await;

// High-frequency SYNC (100 Hz tested successfully)
let mut interval = tokio::time::interval(Duration::from_millis(10));
loop {
    interval.tick().await;
    canopen.send_sync().await?;
}
```

### EMCY (Emergency Messages)

Monitor device errors and warnings:

```rust
// Register emergency handler for specific node
canopen.register_emcy_handler(node_id, |emcy| {
    println!("Emergency from node {}: 0x{:04X} - {}", 
        node_id,
        emcy.error_code, 
        emcy.error_code_description()
    );
    println!("Error register: 0x{:02X}", emcy.error_register);
    println!("Manufacturer data: {:02X?}", emcy.mfr_specific_data);
}).await;

// Get recent emergencies
let recent = canopen.get_recent_emcy(node_id, 10).await;
for emcy in recent {
    println!("Error: {}", emcy.error_code_description());
}
```

### LSS (Layer Setting Services)

Node configuration and commissioning:

```rust
// Switch to configuration mode (all unconfigured slaves)
canopen.lss_switch_state_global(LssMode::Configuration).await?;

// Inquire LSS address
let address = canopen.lss_inquire_lss_address(1000).await?;
println!("Vendor ID: 0x{:08X}", address.vendor_id);
println!("Product Code: 0x{:08X}", address.product_code);
println!("Serial Number: 0x{:08X}", address.serial_number);

// Select specific slave
canopen.lss_switch_state_selective(&address).await?;

// Configure node-ID (example - disabled by default for safety)
match canopen.lss_configure_node_id(10, 1000).await? {
    LssError::Success => println!("Node-ID configured successfully"),
    error => println!("Configuration failed: {}", error.description()),
}

// Store configuration to non-volatile memory
canopen.lss_store_configuration(1000).await?;

// Switch back to waiting mode
canopen.lss_switch_state_global(LssMode::Waiting).await?;
```

## Examples

Comprehensive examples are provided in the `examples/` directory:

- **`sdo_test.rs`** - SDO read/write operations with various data types
- **`nmt_test.rs`** - NMT state control and heartbeat monitoring
- **`pdo_test.rs`** - PDO transmission and reception with callbacks
- **`sync_test.rs`** - SYNC message handling and high-frequency testing
- **`emcy_test.rs`** - Emergency message monitoring and parsing
- **`lss_test.rs`** - LSS commissioning workflow (10 test sections)

Run an example:

```bash
cargo run --release --example sdo_test
cargo run --release --example pdo_test
cargo run --release --example lss_test
```

## Project Status

### ✅ Completed (All Major CANopen Protocols)

- ✅ **Phase 1-3**: Project setup, types, hardware layer
- ✅ **Phase 4**: SDO client (expedited & segmented transfers)
- ✅ **Phase 5**: NMT state management
- ✅ **Phase 6**: Main library and PDO handling  
- ✅ **Phase 7**: SYNC & EMCY protocols
- ✅ **Phase 8**: LSS (Layer Setting Services)
- ✅ **Phase 9**: Documentation and CI/CD pipeline
  - Complete API documentation with examples
  - All public APIs documented with doctests
  - Comprehensive README with protocol examples
  - GitHub Actions CI/CD pipeline

### 🔄 In Progress

- ⏳ **Phase 10**: Advanced testing and benchmarking

### 📋 Future Enhancements (Optional)

- TIME protocol (timestamp synchronization)
- SRDO (Safety-related data objects)
- Block SDO transfers (high-speed bulk data)
- SocketCAN support for Linux


## Architecture

### Module Organization

```
src/
├── lib.rs                 # Main library API and entry point
├── canopen/
│   ├── mod.rs            # CANopen module exports
│   ├── message.rs        # CAN message types and COB-ID definitions
│   ├── sdo.rs            # SDO client with segmented transfer support
│   ├── nmt.rs            # NMT state management and commands
│   ├── pdo.rs            # PDO handling with callbacks
│   ├── sync.rs           # SYNC message handling with counter
│   ├── emcy.rs           # Emergency message processing
│   ├── lss.rs            # LSS protocol implementation
│   ├── od.rs             # Object Dictionary types
│   └── types.rs          # CANopen data type conversions
├── hardware/
│   ├── mod.rs            # Hardware abstraction trait
│   └── peak_can.rs       # PEAK CAN adapter implementation
├── errors.rs             # Comprehensive error types
└── utils.rs              # Utility functions

examples/                  # Comprehensive protocol examples
├── sdo_test.rs
├── nmt_test.rs
├── pdo_test.rs
├── sync_test.rs
├── emcy_test.rs
└── lss_test.rs
```

### Design Patterns

- **Async/Await**: All I/O operations are non-blocking using Tokio
- **Message Passing**: Channels (mpsc, broadcast) for inter-task communication
- **Event-Driven**: Callback system for PDO, heartbeat, SYNC, and EMCY
- **Type Safety**: Strong typing with enums for states, commands, and errors
- **Thread Safety**: Arc + RwLock for shared state across async tasks

## Performance

Tested performance metrics on PEAK CAN USB at 1 Mbps:

- **PDO throughput**: 50+ messages/second sustained
- **SYNC frequency**: 100 Hz (10ms period) tested successfully
- **SDO transfers**: Segmented transfers >4 KB working correctly
- **Latency**: Sub-millisecond message processing in release mode

## Hardware Requirements

### Supported Adapters

- PEAK CAN USB interfaces (PCAN-USB, PCAN-USB Pro, etc.)
- Requires PEAK CAN drivers installed
- Windows support via PCANBasic.dll
- Linux support planned (SocketCAN backend)

### Bus Speeds

All standard CANopen bit rates supported:
- 1 Mbps (default, most common)
- 800 kbps
- 500 kbps
- 250 kbps
- 125 kbps
- 100 kbps
- 50 kbps
- 20 kbps
- 10 kbps

## Error Handling

Comprehensive error types with context:

```rust
pub enum CANopenError {
    Hardware(String),              // CAN hardware errors
    Sdo { code: u32 },            // SDO abort codes
    Timeout,                       // Operation timeout
    InvalidMessage,                // Malformed CAN message
    InvalidLength(usize),          // Data length mismatch
    NodeNotFound { node_id: u8 }, // Node not responding
    ChannelClosed,                 // Internal channel error
    NotInitialized,                // Manager not initialized
    // ... and more
}
```

All async operations return `Result<T>` for proper error propagation.

## Dependencies

Core dependencies:

- `peak-can-sys = "0.1.2"` - PEAK CAN hardware interface
- `tokio` - Async runtime with full features
- `serde` - Serialization with derive macros
- `log = "0.4"` - Logging framework
- `thiserror = "1.0"` - Error handling
- `crossbeam = "0.8"` - Concurrent data structures
- `futures = "0.3"` - Async utilities
- `bitflags = "2.0"` - Bit flag operations
- `async-trait = "0.1"` - Async traits

## Building

### Debug Build (faster compilation)

```bash
cargo build
```

### Release Build (optimized, recommended for performance testing)

```bash
cargo build --release
```

### Running Examples

```bash
# Debug mode (faster to compile, slower to run)
cargo run --example sdo_test

# Release mode (optimized performance)
cargo run --release --example pdo_test
```

### Running Tests

```bash
cargo test
```

**Current Test Status**: ✅ All 161 tests passing
- 150 unit tests (protocol implementation)
- 11 integration tests
- 17 doctests (+ 3 ignored)

CI/CD automatically runs tests on every push via GitHub Actions.

## Logging

Enable logging with environment variables:

```bash
# Windows PowerShell
$env:RUST_LOG="debug"
cargo run --release --example sdo_test

# Linux/macOS
RUST_LOG=debug cargo run --release --example sdo_test
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## Troubleshooting

### Common Issues

**PEAK CAN drivers not found**
```
Solution: Install PEAK CAN drivers from https://www.peak-system.com/
Windows: Ensure PCANBasic.dll is in system PATH
```

**Connection timeout**
```rust
// Check bus speed matches your network
canopen.connect(BusSpeed::Baud1M).await?;

// Verify CAN termination resistors (120Ω at each end)
// Check physical connections
```

**SDO timeout**
```rust
// Increase timeout (default 1000ms)
canopen.sdo_read_with_timeout(node_id, index, subindex, 5000).await?;

// Verify node is in Pre-Operational or Operational state
canopen.nmt_start(node_id).await?;
```

**Build errors**
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

## Contributing

Contributions welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure `cargo test` passes
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## License

MIT License

Copyright (c) 2025 Sicris Rey Embay and Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

See [LICENSE](LICENSE) file for full text.

## Acknowledgments

- Original C# libCanopenSimple library
- PEAK System for CAN hardware and drivers
- Rust community for excellent async ecosystem
- CANopen specification (CiA 301, 305, and device profiles)

## Resources

### CANopen Specifications
- [CiA 301](https://www.can-cia.org/can-knowledge/canopen/canopen/) - CANopen application layer
- [CiA 305](https://www.can-cia.org/can-knowledge/canopen/special-interest-groups/layer-setting-services-lss/) - Layer Setting Services (LSS)

### Related Projects
- [peak-can-sys](https://crates.io/crates/peak-can-sys) - PEAK CAN Rust bindings
- [tokio](https://tokio.rs/) - Async runtime for Rust

## Status & Roadmap

| Feature | Status | Notes |
|---------|--------|-------|
| SDO Client | ✅ Complete | Expedited & segmented |
| NMT Management | ✅ Complete | All commands + heartbeat |
| PDO Handling | ✅ Complete | TX/RX with callbacks |
| SYNC Protocol | ✅ Complete | Counter support, 100Hz tested |
| EMCY Protocol | ✅ Complete | 40+ error codes |
| LSS Protocol | ✅ Complete | All 14 commands |
| PEAK CAN Support | ✅ Complete | All bit rates |
| Documentation | ✅ Complete | Comprehensive docs |
| Unit Tests | ✅ Complete | 161 tests passing |
| CI/CD Pipeline | ✅ Complete | GitHub Actions |
| TIME Protocol | 📋 Future | Optional |
| SRDO | 📋 Future | Optional |
| SocketCAN | 📋 Future | Linux support |

---

**Made with ❤️ using Rust and AI assistance**