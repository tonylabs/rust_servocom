use clap::{Parser, ValueEnum};
use servocom::servo::ServoKind;
use servocom::{DynamixelProtocolHandler, FeetechProtocolHandler};
use std::{error::Error, time::Duration};
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "/dev/serial/by-id/usb-1a86_USB_Single_Serial_5B79034031-if00"
    )]
    serialport: String,
    /// baud
    #[arg(short, long, default_value_t = 1_000_000)]
    baudrate: u32,

    #[arg(short, long, value_enum, default_value_t = ProtocolVersion::V1)]
    protocol: ProtocolVersion,
}

#[derive(ValueEnum, Clone, Debug)]
enum ProtocolVersion {
    V1,
    V2,
    Feetech,
}

/// Brand-specific transport so the scanner can dispatch ping/read against the
/// right protocol handler without conflating Dynamixel and Feetech APIs.
enum Scanner {
    Dynamixel(DynamixelProtocolHandler),
    Feetech(FeetechProtocolHandler),
}

impl Scanner {
    fn ping(&self, port: &mut dyn serialport::SerialPort, id: u8) -> Result<bool, Box<dyn Error>> {
        match self {
            Scanner::Dynamixel(dph) => dph.ping(port, id),
            Scanner::Feetech(fph) => fph.ping(port, id),
        }
    }

    fn read(
        &self,
        port: &mut dyn serialport::SerialPort,
        id: u8,
        addr: u8,
        length: u8,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        match self {
            Scanner::Dynamixel(dph) => dph.read(port, id, addr, length),
            Scanner::Feetech(fph) => fph.read(port, id, addr, length),
        }
    }

    /// Address holding the 2-byte model identifier for this brand.
    ///
    /// Dynamixel servos expose the model number at 0x00 (`Model Number`).
    /// Feetech servos do not have a model-number register; the closest match
    /// in the FT-SCS memory table is the Servo Major/Minor Version pair at
    /// 0x03–0x04, which uniquely identifies the product family (e.g. SM40BL
    /// reports [0x08, 0x28]).
    fn model_addr(&self) -> u8 {
        match self {
            Scanner::Dynamixel(_) => 0,
            Scanner::Feetech(_) => 3,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let serialport: String = args.serialport;
    let baudrate: u32 = args.baudrate;
    let protocol: ProtocolVersion = args.protocol;

    println!("Scanning for servos on {serialport} at {baudrate} baud using {protocol:?}");

    println!("Scanning...");
    let mut serial_port = serialport::new(serialport, baudrate)
        .timeout(Duration::from_millis(10))
        .open()?;

    let scanner = match protocol {
        ProtocolVersion::V1 => Scanner::Dynamixel(DynamixelProtocolHandler::v1()),
        ProtocolVersion::V2 => Scanner::Dynamixel(DynamixelProtocolHandler::v2()),
        ProtocolVersion::Feetech => Scanner::Feetech(FeetechProtocolHandler::new()),
    };

    let model_addr = scanner.model_addr();

    use std::io::Write;
    for id in 1..253 {
        match scanner.ping(serial_port.as_mut(), id) {
            Ok(true) => {
                let model = scanner.read(serial_port.as_mut(), id, model_addr, 2).unwrap();
                let model = u16::from_le_bytes([model[0], model[1]]);
                let kind = ServoKind::try_from(model);
                match kind {
                    Ok(m) => println!("\nFound motor with id {id} and model: {m:?}"),
                    Err(e) => println!("\nFound motor with id {id} with {e:?} (raw: {model})"),
                }
            }
            Ok(false) => {
                eprint!(".");
                std::io::stderr().flush().ok();
            }
            Err(e) => eprintln!("\nid {id}: {e}"),
        };
    }
    eprintln!();

    Ok(())
}
