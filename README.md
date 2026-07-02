# ServoCom: a Rust package to communicate with Dynamixel/Feetech motors

## Getting started

ServoCom is a communication library for Dynamixel/Feetech motors.

### Attribution and Acknowledgements

This project is a derivative work based on [rustypot](https://github.com/pollen-robotics/rustypot), originally developed and maintained by [Pollen Robotics](https://pollen-robotics.com). It has been modified and extended from the upstream source.

Both the original work and this derivative are distributed under the [Apache License 2.0](./LICENSE). In accordance with Section 4 of the License, all original copyright, patent, trademark, and attribution notices from the source form have been retained, and this notice is provided to indicate that modifications have been made to the original files.

We sincerely thank the rustypot authors and the Pollen Robotics team for releasing their work as open source, which made this project possible.

## Feature Overview

* Relies on [serialport](https://docs.rs/serialport/latest/serialport/) for serial communication
* Support for Dynamixel protocol v1 and v2
* Support for Feetech FT-SCS protocol
* Support for sync read and sync write operations
* Pure Rust plus python bindings (using [pyo3](https://pyo3.rs/)).

To add new servo, please refer to the [Servo documentation](./src/servo/README.md).

## APIs

It exposes two layers:
* Low-level protocol handlers: handle the serial communication and packet parsing. Useful for fine-grained control of a shared bus.
  * `DynamixelProtocolHandler` — Dynamixel Protocol 1.0 / 2.0 (constructed with `::v1()` or `::v2()`).
  * `FeetechProtocolHandler` — Feetech FT-SCS protocol (constructed with `::new()`; Feetech only has one protocol version).
* `Controller`: high-level API per servo. Simpler and cleaner but takes full ownership of the io (can still be shared if wrapped with a mutex). Dynamixel controllers select their protocol version via `.with_protocol_v1()` / `.with_protocol_v2()`. Feetech controllers attach the handler automatically in `new()`.

See the examples below for usage.

### Examples

```rust
use servocom::servo::feetech::sts3215::STS3215Controller;
use std::time::Duration;

fn main() {
    let serial_port = serialport::new("/dev/ttyACM0", 1_000_000)
        .timeout(Duration::from_millis(1000))
        .open()
        .unwrap();

    let mut c = STS3215Controller::new()
            .with_serial_port(serial_port);

    let pos = c.sync_read_present_position(&vec![1, 2]).unwrap();
    println!("Motors present position: {:?}", pos);

    c.sync_write_goal_position(&vec![1, 2], &vec![1000, 2000]).unwrap();
}
```

STS3025BL is another servo in the STS family and uses the same TTL single-bus (default 1 Mbps):

```rust
use servocom::servo::feetech::sts3025bl::STS3025BLController;
use std::time::Duration;

fn main() {
    let serial_port = serialport::new("/dev/ttyACM0", 1_000_000)
        .timeout(Duration::from_millis(1000))
        .open()
        .unwrap();

    let mut c = STS3025BLController::new()
            .with_serial_port(serial_port);

    let pos = c.sync_read_present_position(&vec![1, 2]).unwrap();
    println!("Motors present position: {:?}", pos);

    c.sync_write_goal_position(&vec![1, 2], &vec![1000, 2000]).unwrap();
}
```

SM40BL belongs to the SMS magnetic-encoder family. Note the different default baud rate (115200) and that it uses RS485:

```rust
use servocom::servo::feetech::sm40bl::SM40BLController;
use std::time::Duration;

fn main() {
    let serial_port = serialport::new("/dev/ttyACM0", 115_200)
        .timeout(Duration::from_millis(1000))
        .open()
        .unwrap();

    let mut c = SM40BLController::new()
            .with_serial_port(serial_port);

    let pos = c.sync_read_present_position(&vec![1, 2]).unwrap();
    println!("Motors present position: {:?}", pos);

    c.sync_write_goal_position(&vec![1, 2], &vec![1000, 2000]).unwrap();
}
```

Low-level Feetech access uses `FeetechProtocolHandler` directly:

```rust
use servocom::{FeetechProtocolHandler, servo::feetech::sts3215};
use std::time::Duration;

fn main() {
    let mut serial_port = serialport::new("/dev/ttyUSB0", 1_000_000)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open port");

    let fph = FeetechProtocolHandler::new();

    let pos = sts3215::read_present_position(&fph, serial_port.as_mut(), 1)
        .expect("Communication error");
    println!("Motor STS3215 ID: 1 present position: {:?}", pos);
}
```

```rust
use servocom::{DynamixelProtocolHandler, servo::dynamixel::mx};
use std::time::Duration;

fn main() {
    let mut serial_port = serialport::new("/dev/ttyACM0", 1_000_000)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open port");

    let dph = DynamixelProtocolHandler::v1();

    loop {
        let pos =
            mx::read_present_position(&dph, serial_port.as_mut(), 11).expect("Communication error");
        println!("Motor 11 present position: {:?}", pos);
    }
}
```

## Tools

Simple bus scanning tool. The `--protocol` flag selects which handler to use; `feetech` reads the model identifier from address `0x03` (Servo Major/Minor) instead of `0x00`.

```bash
# Dynamixel Protocol 1.0 (AX, MX, ...)
cargo run --bin=scan -- --serialport=/dev/ttyACM0 --baudrate=1000000 --protocol=v1

# Dynamixel Protocol 2.0 (XL320, XL330, XL430, ...)
cargo run --bin=scan -- --serialport=/dev/ttyACM0 --baudrate=1000000 --protocol=v2

# Feetech FT-SCS (STS, SCS, SM families)
cargo run --bin=scan -- --serialport=/dev/ttyUSB0 --baudrate=1000000 --protocol=feetech

# SMS / SCS servos use 115200 baud by default
cargo run --bin=scan -- --serialport=/dev/ttyUSB0 --baudrate=115200 --protocol=feetech
```

## Python bindings

The python bindings are generated using [pyo3](https://pyo3.rs/). They are available on `pypi`(https://pypi.org/project/servocom/). You can install them using pip.

```bash
pip install servocom
```

To build them locally, you can use [maturin](https://www.maturin.rs).

First, generate the type annotations for the python bindings, by running:

```bash
cargo run --release --bin stub_gen --features python
```

Then, you can build the python bindings using maturin. You can either build the wheel files to distribute them, or install them directly in your local python environment.

To build the wheel files, you can run:

```bash
maturin build --release --features python --features pyo3/extension-module
```

or, if you want to install them in your local python environment:

```bash
maturin develop --release --features python --features pyo3/extension-module
```

See [maturin official documentation](https://maturin.rs) for more information on how to use it.

Rebuild everything:

```bash
cargo clean && cargo build --bin scan
```

### Using the Python bindings

The Python bindings exposes the same API as the Controller API in the rust crate.

You first need to create a Controller object. For instance, to communicate with a serial port to Feetech STS3215 motors, you can do the following:

```python
from servocom import Sts3215PyController

c = Sts3215PyController(serial_port='/dev/ttyUSB0', baudrate=1_000_000, timeout=0.1)
```


Then, you can directly read/write any register of the motor. For instance, to read the present position of the motor with id 1, you can do:

```python

pos = c.read_present_position(1)
print(pos)
```

You can also write to the motors. For instance, to set the goal position of the motors with id 1 to 90° you can do:

```python
import numpy as np
c.write_goal_position(1, np.deg2rad(90.0))
```


Then, you can also sync_read any registers on multiple motors in a single operations. For instance, to read the present position of the motors with id 1 and 2, you can do:

```python

pos = c.sync_read_present_position([1, 2])
print(pos)
```

Same with sync_write. For instance, to set the goal position of the motors with id 1 and 2 to 0.0 and 90° respectively, you can do:

```python
import numpy as np
c.sync_write_goal_position([1, 2], [0.0, np.deg2rad(90.0)])
```

## License

This library is licensed under the [Apache License 2.0](./LICENSE).