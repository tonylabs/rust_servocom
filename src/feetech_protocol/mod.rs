use serialport::SerialPort;
mod packet;
use packet::{InstructionPacket, Packet, StatusPacket};
mod feetech;
use feetech::FtScs;

use crate::Result;

#[derive(Debug)]
/// Raw Feetech (FT-SCS) communication messages controller.
/// Feetech servos use a single protocol version
pub struct FeetechProtocolHandler {
    protocol: FtScs,
    post_delay: Option<Duration>,
}

impl Default for FeetechProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FeetechProtocolHandler {
    /// Creates a new Feetech FT-SCS communication IO.
    ///
    /// # Examples
    /// ```no_run
    /// use servocom::{FeetechProtocolHandler, servo::feetech::sts3215};
    /// use std::time::Duration;
    ///
    /// let mut serial_port = serialport::new("/dev/ttyUSB0", 1_000_000)
    ///     .timeout(Duration::from_millis(10))
    ///     .open()
    ///     .expect("Failed to open port");
    ///
    /// let fph = FeetechProtocolHandler::new();
    ///
    /// let pos = sts3215::read_present_position(&fph, serial_port.as_mut(), 1)
    ///     .expect("Communication error");
    /// println!("Motor STS3215 ID: 1 present position: {:?}", pos);
    /// ```
    pub fn new() -> Self {
        FeetechProtocolHandler {
            protocol: FtScs,
            post_delay: None,
        }
    }

    /// Set a delay after each communication.
    pub fn with_post_delay(self, delay: Duration) -> Self {
        FeetechProtocolHandler {
            post_delay: Some(delay),
            ..self
        }
    }

    /// Send a ping instruction.
    ///
    /// Ping the motor with specified `id`.
    /// Returns a [CommunicationErrorKind] if the communication fails.
    pub fn ping(&self, serial_port: &mut dyn serialport::SerialPort, id: u8) -> Result<bool> {
        self.protocol.ping(serial_port, id)
    }

    /// Send a reboot instruction.
    ///
    /// Note: not all Feetech servos implement this instruction; if the servo
    /// does not respond the call will time out.
    pub fn reboot(&self, serial_port: &mut dyn serialport::SerialPort, id: u8) -> Result<bool> {
        self.protocol.reboot(serial_port, id)
    }

    /// Factory reset instruction.
    ///
    /// Reset the control table of the servo to its factory default values.
    /// `conserve_id_only` and `conserve_id_and_baudrate` are not supported by
    /// the FT-SCS protocol and will return an [`CommunicationErrorKind::Unsupported`]
    /// error if either is set.
    pub fn factory_reset(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        id: u8,
        conserve_id_only: bool,
        conserve_id_and_baudrate: bool,
    ) -> Result<()> {
        if conserve_id_only || conserve_id_and_baudrate {
            return Err(Box::new(CommunicationErrorKind::Unsupported));
        }
        self.protocol
            .factory_reset(serial_port, id, conserve_id_only, conserve_id_and_baudrate)
    }

    /// Reads raw register bytes.
    pub fn read(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        id: u8,
        addr: u8,
        length: u8,
    ) -> Result<Vec<u8>> {
        let res = self.protocol.read(serial_port, id, addr, length);
        if let Some(delay) = self.post_delay {
            std::thread::sleep(delay);
        }
        res
    }

    /// Writes raw bytes to register.
    pub fn write(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        id: u8,
        addr: u8,
        data: &[u8],
    ) -> Result<()> {
        self.protocol.write(serial_port, id, addr, data)?;
        if let Some(delay) = self.post_delay {
            std::thread::sleep(delay);
        }
        Ok(())
    }

    /// Writes raw bytes to register and returns the status packet payload.
    pub fn write_fb(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        id: u8,
        addr: u8,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        let res = self.protocol.write_fb(serial_port, id, addr, data);
        if let Some(delay) = self.post_delay {
            std::thread::sleep(delay);
        }
        res
    }

    /// Reads raw register bytes from multiple ids at once.
    ///
    /// *Note: sync read support depends on the usb to serial hardware used!*
    pub fn sync_read(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        ids: &[u8],
        addr: u8,
        length: u8,
    ) -> Result<Vec<Vec<u8>>> {
        self.protocol.sync_read(serial_port, ids, addr, length)
    }

    /// Write raw bytes to multiple ids at once.
    pub fn sync_write(
        &self,
        serial_port: &mut dyn serialport::SerialPort,
        ids: &[u8],
        addr: u8,
        data: &[Vec<u8>],
    ) -> Result<()> {
        self.protocol.sync_write(serial_port, ids, addr, data)
    }
}

trait Protocol<P: Packet> {
    fn ping(&self, port: &mut dyn SerialPort, id: u8) -> Result<bool> {
        self.send_instruction_packet(port, P::ping_packet(id).as_ref())?;

        Ok(self.read_status_packet(port, id).is_ok())
    }

    fn reboot(&self, port: &mut dyn SerialPort, id: u8) -> Result<bool> {
        self.send_instruction_packet(port, P::reboot_packet(id).as_ref())?;

        Ok(self.read_status_packet(port, id).is_ok())
    }

    fn factory_reset(
        &self,
        port: &mut dyn SerialPort,
        id: u8,
        conserve_id_only: bool,
        conserve_id_and_baudrate: bool,
    ) -> Result<()> {
        self.send_instruction_packet(
            port,
            P::factory_reset_packet(id, conserve_id_only, conserve_id_and_baudrate).as_ref(),
        )?;
        self.read_status_packet(port, id).map(|_| ())
    }

    fn read(&self, port: &mut dyn SerialPort, id: u8, addr: u8, length: u8) -> Result<Vec<u8>> {
        self.send_instruction_packet(port, P::read_packet(id, addr, length).as_ref())?;
        self.read_status_packet(port, id)
            .map(|sp| sp.params().to_vec())
    }
    fn write(&self, port: &mut dyn SerialPort, id: u8, addr: u8, data: &[u8]) -> Result<()> {
        self.send_instruction_packet(port, P::write_packet(id, addr, data).as_ref())?;
        self.read_status_packet(port, id).map(|_| ())
    }

    fn write_fb(
        &self,
        port: &mut dyn SerialPort,
        id: u8,
        addr: u8,
        data: &[u8],
    ) -> Result<Vec<u8>> {
        self.send_instruction_packet(port, P::write_packet(id, addr, data).as_ref())?;
        self.read_status_packet(port, id)
            .map(|sp| sp.params().to_vec())
    }

    fn sync_read(
        &self,
        port: &mut dyn SerialPort,
        ids: &[u8],
        addr: u8,
        length: u8,
    ) -> Result<Vec<Vec<u8>>> {
        self.send_instruction_packet(port, P::sync_read_packet(ids, addr, length).as_ref())?;
        let mut result = Vec::new();
        for id in ids {
            let sp = self.read_status_packet(port, *id)?;
            result.push(sp.params().to_vec());
        }
        Ok(result)
    }
    fn sync_write(
        &self,
        port: &mut dyn SerialPort,
        ids: &[u8],
        addr: u8,
        data: &[Vec<u8>],
    ) -> Result<()> {
        self.send_instruction_packet(port, P::sync_write_packet(ids, addr, data).as_ref())?;
        Ok(())
    }

    const MAX_FLUSH_RETRIES: usize = 3;
    const FLUSH_RETRY_DELAY_MS: u64 = 5;

    fn flush_if_needed(
        &self,
        port: &mut dyn SerialPort,
        max_retries: usize,
        flush_after_delay: Duration,
    ) -> Result<()> {
        for _attempt in 1..=max_retries {
            if self.is_input_buffer_empty(port)? {
                return Ok(());
            }
            self.flush(port)?;
            std::thread::sleep(flush_after_delay);
        }
        if self.is_input_buffer_empty(port)? {
            Ok(())
        } else {
            Err(Box::new(CommunicationErrorKind::TimeoutError))
        }
    }

    fn send_instruction_packet(
        &self,
        port: &mut dyn SerialPort,
        packet: &dyn InstructionPacket<P>,
    ) -> Result<()> {
        self.flush_if_needed(
            port,
            Self::MAX_FLUSH_RETRIES,
            Duration::from_millis(Self::FLUSH_RETRY_DELAY_MS),
        )?;

        match port.write_all(&packet.to_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(Box::new(CommunicationErrorKind::TimeoutError)),
        }
    }
    fn read_status_packet(
        &self,
        port: &mut dyn SerialPort,
        sender_id: u8,
    ) -> Result<Box<dyn StatusPacket<P>>> {
        let mut header = vec![0u8; P::HEADER_SIZE];
        port.read_exact(&mut header)?;

        let payload_size = P::get_payload_size(&header)?;
        let mut payload = vec![0u8; payload_size];
        port.read_exact(&mut payload)?;

        let mut data = Vec::new();
        data.extend(header);
        data.extend(payload);

        P::status_packet(&data, sender_id)
    }

    fn is_input_buffer_empty(&self, port: &mut dyn SerialPort) -> Result<bool> {
        let n = port.bytes_to_read()? as usize;
        Ok(n == 0)
    }

    fn flush(&self, port: &mut dyn SerialPort) -> Result<()> {
        let n = port.bytes_to_read()? as usize;
        if n > 0 {
            let mut buff = vec![0u8; n];
            port.read_exact(&mut buff)?;
        }

        Ok(())
    }
}

use std::{fmt, time::Duration};

/// Feetech Communication Error
#[derive(Debug, Clone, Copy)]
pub enum CommunicationErrorKind {
    /// Incorrect checksum
    ChecksumError,
    /// Could not parse incoherent message
    ParsingError,
    /// Timeout
    TimeoutError,
    /// Incorrect response id - different from sender (sender id, response id)
    #[allow(dead_code)]
    IncorrectId(u8, u8),

    /// Operation not supported
    Unsupported,
}
impl fmt::Display for CommunicationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommunicationErrorKind::ChecksumError => write!(f, "Checksum error"),
            CommunicationErrorKind::ParsingError => write!(f, "Parsing error"),
            CommunicationErrorKind::TimeoutError => write!(f, "Timeout error"),
            CommunicationErrorKind::IncorrectId(sender_id, resp_id) => {
                write!(f, "Incorrect id ({resp_id} instead of {sender_id})")
            }
            CommunicationErrorKind::Unsupported => write!(f, "Operation not supported"),
        }
    }
}
impl std::error::Error for CommunicationErrorKind {}
