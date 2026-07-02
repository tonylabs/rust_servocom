# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`servocom` is a Rust crate (with Python bindings via PyO3) for talking to Dynamixel and Feetech serial-bus servos.

## Common commands

```bash
# Default Rust build / test / format (CI runs all three on PRs)
cargo build
cargo test
cargo fmt --all -- --check

# Build with the python feature (this is the build that has historically broken — verify before releasing)
cargo build --features python

# Bus scanner (see README for examples per protocol)
cargo run --bin=scan -- --serialport=/dev/ttyACM0 --baudrate=1000000 --protocol=v1|v2|feetech

# Examples (each maps to a file in examples/)
cargo run --example feetech_controller
cargo run --example dxl_mx_example

# Python bindings — generate stubs first, then build/install
cargo run --release --bin stub_gen --features python
maturin develop --release --features python --features pyo3/extension-module   # local install
maturin build   --release --features python --features pyo3/extension-module   # wheel

# Single test
cargo test --lib offset_conversions
```

`stub_gen` regenerates `.pyi` files for the Python API and **must** be run before `maturin build` / publishing wheels — the `python.yml` workflow runs it as a separate step on every target.

## Architecture

The crate is layered. Understanding the macro pipeline is essential before changing anything in `src/servo/`.

### Two API layers
1. **Protocol handlers** (`src/dynamixel_protocol/`, `src/feetech_protocol/`) — own packet framing, checksums, ping/read/write/sync_read/sync_write. Take a `&mut dyn serialport::SerialPort` per call so a bus can be shared.
   - `DynamixelProtocolHandler::v1()` / `::v2()` — Dynamixel Protocol 1.0 and 2.0 dispatch through a `ProtocolKind` enum.
   - `FeetechProtocolHandler::new()` — single FT-SCS protocol; both handlers implement an internal `Protocol<P: Packet>` trait that provides default ping/read/write logic with packet-type indirection.
2. **Per-servo `*Controller` structs** — high-level wrappers that own the serial port + handler. Constructed via `<ServoName>Controller::new().with_serial_port(port)`; Dynamixel controllers additionally need `.with_protocol_v1()` or `.with_protocol_v2()` (Feetech does not — protocol is auto-attached in `new()`).

### The servo macro pipeline (`src/servo/servo_macro.rs`)
Every concrete servo file (e.g. `src/servo/feetech/sts3215.rs`, `src/servo/dynamixel/mx.rs`) is a thin call to `generate_servo!(Name, <v1|v2|feetech>, reg: (name, r|w|rw, addr, type, conv), ...)`. The macro chain expands to:
- A `<Name>Controller` struct with `read_X`/`write_X`/`sync_read_X`/`sync_write_X` per register, plus `read_raw_X`/`write_raw_X` variants when a conversion is set.
- Free functions (`<servo_module>::read_X(io, port, id)`) for low-level use.
- A `<Name>PyController` PyO3 wrapper (gated on `feature = "python"`), wrapping the Rust controller in a `Mutex` so it can be `frozen`.
- `ping`/`reboot`/`factory_reset` and raw `read/write/sync_read/sync_write_raw_data` (for any address) via `generate_special_instructions!` and `generate_addr_read_write!`.

The `conv` slot is either `None` (raw bytes only) or a type implementing `Conversion` (`src/servo/conversion.rs`) — e.g. `AnglePosition`, `Velocity`, `Offset`, or the built-in `bool` impl. Conversion types define `RegisterType` ↔ `UsiType` (typically `i16`/`u16` ↔ `f64` radians).

### Servo registry (`src/servo/mod.rs`)
The `register_servo!` invocation at the bottom of `mod.rs` is the single source of truth that:
1. Builds the `ServoKind` enum and `ServoKind::try_from(model_number: u16)` used by the `scan` binary to identify detected motors.
2. Registers every `*PyController` class with the Python module in `register_class()`.

When adding a new servo:
1. Create `src/servo/<family>/<name>.rs` invoking `generate_servo!`.
2. Declare the module in `src/servo/<family>/mod.rs`.
3. Add a `servo: (<family>, <Name>, (<Variant>, <model_number>))` entry in `src/servo/mod.rs` so it's discoverable by scan and exported to Python.

See `src/servo/README.md` for the same workflow in narrative form.

### Scan binary
`src/bin/scan.rs` reads model identifiers at different addresses depending on protocol — Dynamixel uses `0x00` (Model Number, 2 bytes), Feetech uses `0x03` (Servo Major/Minor Version) since FT-SCS has no model-number register. Keep this dispatch logic in mind when extending scan.

### Default baud rates

Servo families use different defaults: STS / SCS-family Feetech runs at 1 Mbps over TTL single-bus; SMS-family (e.g. SM40BL) runs at 115200 baud over RS485. Examples in `README.md` reflect this — preserve those defaults when adding examples.

---

## Releasing

The release process is CI-driven (push tag → publish.yml runs cargo publish + PyPI), so never run cargo publish. Just version bump + commit + push + tag.

Help user to summerize commit messages for group of changes.

Always ask user these two question when user want to release a new version:

- Which changes should be included in this release? (Only Claude code changes / Include everything currently in the tree)
- What version bump from? (minor / patch / major)