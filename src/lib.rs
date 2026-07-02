//! A low-level communication library for servo (Dynamixel and Feetech motors).
//!
//! ## Attribution
//!
//! This crate is a derivative work based on
//! [servocom](https://github.com/pollen-robotics/servocom), originally
//! developed and maintained by [Pollen Robotics](https://pollen-robotics.com).
//! It has been modified and extended from the upstream source.
//!
//! Both the original work and this derivative are distributed under the
//! [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). In
//! accordance with Section 4 of the License, all original copyright, patent,
//! trademark, and attribution notices from the source form have been retained,
//! and this notice indicates that modifications have been made to the
//! original files. See the `NOTICE` file distributed with this crate for
//! additional details.
//!
//! We sincerely thank the servocom authors and the Pollen Robotics team for
//! releasing their work as open source.
//!
//! ## Feature Overview
//!
//! * Relies on [serialport] for serial communication
//! * Support for dynamixel protocol v1 and v2 (both can be used on the same io)
//! * Support for sync read and sync write operations
//! * Easy support for new type of motors (register definition through macros)
//! * Pure Rust
//!
//! ## APIs
//!
//! It exposes two APIs:
//! * `DynamixelProtocolHandler`: low-level API. It handles the serial communication and the Dynamixel protocol parsing. It can be used for fine-grained control of the shared bus with other communication.
//! * `Controller`: high-level API for the Dynamixel protocol. Simpler and cleaner API but it takes full ownership of the io (it can still be shared if wrapped with a mutex for instance).
//!
//! See the examples below for usage.
//!
//! ## Examples
//!
//! ### With the low-level API
//! ```no_run
//! use servocom::{DynamixelProtocolHandler, servo::dynamixel::mx};
//! use std::time::Duration;
//!
//! let mut port = serialport::new("/dev/ttyACM0", 1_000_000)
//!     .timeout(Duration::from_millis(10))
//!     .open()
//!     .expect("Failed to open port");
//!
//! let dph = DynamixelProtocolHandler::v1();
//!
//! let raw_pos: i16 = mx::read_raw_present_position(&dph, port.as_mut(), 11).expect("Communication error");
//! let pos: f64 =
//!     mx::read_present_position(&dph, port.as_mut(), 11).expect("Communication error");
//! println!("Motor 11 present position: {:?}rads (raw: {:?})", pos, raw_pos);
//! ```
//!
//! ### With the high-level API
//! ```no_run
//! use servocom::servo::feetech::sts3215::Sts3215Controller;
//! use std::time::Duration;
//!
//! let port = serialport::new("/dev/ttyUSB0", 1_000_000)
//!     .timeout(Duration::from_millis(1000))
//!     .open()
//!     .unwrap();
//!
//! let mut c = Sts3215Controller::new().with_serial_port(port);
//!
//! let pos = c.sync_read_present_position(&vec![1, 2]).unwrap();
//! println!("Motors present position: {:?}", pos);
//!
//! c.sync_write_goal_position(&vec![1, 2], &vec![0.0, 90.0_f64.to_radians()]).unwrap();
//! ```

pub mod servo;

mod dynamixel_protocol;
pub use dynamixel_protocol::{CommunicationErrorKind, DynamixelProtocolHandler};

mod feetech_protocol;
pub use feetech_protocol::FeetechProtocolHandler;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3_stub_gen::define_stub_info_gatherer;

#[cfg(feature = "python")]
#[pymodule]
fn servocom(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    servo::register_class(m)?;

    Ok(())
}

#[cfg(feature = "python")]
define_stub_info_gatherer!(stub_info);
