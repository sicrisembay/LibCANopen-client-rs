# libCANopen Simple (Rust)

A Rust implementation of a simple CANopen library using PEAK CAN hardware adapters.

## Project Status

✅ **Phase 1 Complete**: Project Setup and Dependencies
- [x] Cargo project initialization
- [x] Dependencies configuration (peak-can-sys 0.1.2, tokio, etc.)
- [x] Directory structure setup
- [x] Basic module skeleton with placeholders
- [x] Compiling library with example
- [x] Unit tests passing

🔄 **Next Steps**: Phase 2 & 3 - Core Data Types and Hardware Implementation

## Features (Planned)

- Async/await based API
- SDO client with expedited and segmented transfers  
- NMT state management and commands
- PDO handling
- Event-driven architecture
- PEAK CAN hardware support via peak-can-sys

## Current Structure

```
src/
├── lib.rs                 # Main library entry point ✅
├── canopen/
│   ├── mod.rs             # CANopen module ✅
│   ├── message.rs         # CAN message types and COB definitions ✅
│   ├── sdo.rs             # SDO client (placeholder) ✅
│   ├── nmt.rs             # NMT management (placeholder) ✅
│   ├── pdo.rs             # PDO handling (placeholder) ✅
│   └── events.rs          # Event system ✅
├── hardware/
│   ├── mod.rs             # Hardware abstraction layer ✅
│   └── peak_can.rs        # PEAK CAN adapter (placeholder) ✅
├── errors.rs              # Error types ✅
└── utils.rs               # Utility functions ✅
```

## Quick Start (Phase 1)

```bash
# Build the project
cargo build

# Run tests
cargo test

# Check example compilation
cargo check --example basic_usage
```

## Dependencies

- `peak-can-sys = "0.1.2"` - PEAK CAN hardware interface
- `tokio` - Async runtime with full features
- `serde` - Serialization with derive macros
- `log = "0.4"` - Logging framework
- `thiserror = "1.0"` - Error handling
- `crossbeam = "0.8"` - Concurrent data structures
- `futures = "0.3"` - Async utilities
- `bitflags = "2.0"` - Bit flag operations
- `async-trait = "0.1"` - Async traits

## Migration Progress

This is a migration from a C# CANopen library. See [MIGRATION_PLAN.md](MIGRATION_PLAN.md) for the complete migration roadmap.

### Completed:
- ✅ Phase 1: Project Setup and Dependencies

### Upcoming:
- 🔄 Phase 2: Core Data Types and Message Handling
- 🔄 Phase 3: Hardware Abstraction Layer (PEAK CAN implementation)
- 🔄 Phase 4: SDO Client Implementation  
- 🔄 Phase 5: NMT State Management
- 🔄 Phase 6: Main Library Implementation
- 🔄 Phase 7: Testing and Examples
- 🔄 Phase 8: Documentation
- 🔄 Phase 9: Migration Validation
- 🔄 Phase 10: Advanced Features

## License

GPL-3.0 (matching the original C# implementation)

## Contributing

This is a migration project. Please refer to the migration plan for implementation guidelines and phase-by-phase development approach.