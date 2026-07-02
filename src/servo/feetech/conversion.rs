//! FeeTech STS/SM register decodings.
//!
//! These conversions translate the FeeTech control table's quirky encodings
//! (BIT15 direction flags, BIT10 direction flags, 4-segment sign-magnitude
//! offsets) into plain signed integers in the documented physical ranges.
//!
//! Pair each one with `read_*` / `write_*` (decoded) — the matching
//! `read_raw_*` / `write_raw_*` methods are byte-for-byte identical to the
//! on-the-wire register value and bypass these conversions entirely.

use std::error::Error;

use crate::servo::conversion::{Conversion, ConversionRangeError};

/// Present Position decoding (register 0x38).
///
/// The 16-bit raw value's BIT15 is a direction flag (not a sign bit) and the
/// physical encoder is 12-bit. We mask to the low 12 bits to recover the
/// in-rotation encoder count in `[0, 4095]` covering 0°–360°.
///
/// `to_raw` is provided for symmetry but the register is read-only on real
/// hardware; values outside `[0, 4095]` are rejected.
pub struct Position;

impl Conversion for Position {
    type RegisterType = u16;
    type UsiType = i32;

    fn from_raw(raw: u16) -> i32 {
        (raw & 0x0FFF) as i32
    }

    fn to_raw(value: i32) -> Result<u16, Box<dyn Error>> {
        if !(0..=4095).contains(&value) {
            return Err(Box::new(ConversionRangeError {
                what: "present_position",
                value: value as i64,
                min: 0,
                max: 4095,
            }));
        }
        Ok(value as u16)
    }
}

/// FeeTech 4-segment sign-magnitude encoding used by Position Offset (0x1F)
/// and Min/Max Angle Limit (0x09 / 0x0B).
///
/// Raw 16-bit register layout:
///
/// ```text
///     0..=2047    →     0..=+2047
///  2048..=4095    →     0..=-2047
///  4096..=6143    →  +2048..=+4095
///  6144..=8191    →  -2048..=-4095
/// ```
///
/// Decoded range is therefore `[-4095, +4095]`.
pub struct Offset;

impl Conversion for Offset {
    type RegisterType = u16;
    type UsiType = i32;

    fn from_raw(raw: u16) -> i32 {
        match raw {
            0..=2047 => raw as i32,
            2048..=4095 => -((raw - 2048) as i32),
            4096..=6143 => (raw - 4096 + 2048) as i32,
            6144..=8191 => -((raw - 6144 + 2048) as i32),
            _ => (raw & 0x0FFF) as i32, // upper bits beyond the documented table — fall back to low 12 bits
        }
    }

    fn to_raw(value: i32) -> Result<u16, Box<dyn Error>> {
        if !(-4095..=4095).contains(&value) {
            return Err(Box::new(ConversionRangeError {
                what: "offset",
                value: value as i64,
                min: -4095,
                max: 4095,
            }));
        }
        let abs = value.unsigned_abs() as u16;
        let raw = if value >= 0 {
            if abs <= 2047 {
                abs
            } else {
                4096 + (abs - 2048)
            }
        } else if abs <= 2047 {
            2048 + abs
        } else {
            6144 + (abs - 2048)
        };
        Ok(raw)
    }
}

/// Present Speed decoding (register 0x3A).
///
/// BIT15 is the sign bit (sign-magnitude, NOT two's complement). The low 15
/// bits are the magnitude. Decoded value stays in the register's native units
/// (0.0146 RPM or 0.732 RPM depending on the `phase` register's step setting).
/// We do not convert to RPM here.
pub struct SignedSpeed;

impl Conversion for SignedSpeed {
    type RegisterType = u16;
    type UsiType = i32;

    fn from_raw(raw: u16) -> i32 {
        let magnitude = (raw & 0x7FFF) as i32;
        if raw & 0x8000 != 0 {
            -magnitude
        } else {
            magnitude
        }
    }

    fn to_raw(value: i32) -> Result<u16, Box<dyn Error>> {
        if !(-32767..=32767).contains(&value) {
            return Err(Box::new(ConversionRangeError {
                what: "present_speed",
                value: value as i64,
                min: -32767,
                max: 32767,
            }));
        }
        let magnitude = value.unsigned_abs() as u16;
        Ok(if value < 0 {
            magnitude | 0x8000
        } else {
            magnitude
        })
    }
}

/// Present Load decoding (register 0x3C).
///
/// BIT10 is the sign bit (not BIT15) and the low 10 bits are the magnitude.
/// Decoded value is `[-1023, +1023]` in 0.1% steps of duty cycle.
pub struct SignedLoad;

impl Conversion for SignedLoad {
    type RegisterType = u16;
    type UsiType = i32;

    fn from_raw(raw: u16) -> i32 {
        let magnitude = (raw & 0x03FF) as i32;
        if raw & 0x0400 != 0 {
            -magnitude
        } else {
            magnitude
        }
    }

    fn to_raw(value: i32) -> Result<u16, Box<dyn Error>> {
        if !(-1023..=1023).contains(&value) {
            return Err(Box::new(ConversionRangeError {
                what: "present_load",
                value: value as i64,
                min: -1023,
                max: 1023,
            }));
        }
        let magnitude = value.unsigned_abs() as u16;
        Ok(if value < 0 {
            magnitude | 0x0400
        } else {
            magnitude
        })
    }
}

/// Angular velocity encoding used for goal_speed (FeeTech STS/SM).
///
/// Kept for the writable goal_speed register, which still uses radians-per-
/// second as its USI unit and the FeeTech "sign-bit = direction" encoding on
/// the wire. Out-of-range values produce a range error rather than wrapping.
pub struct Velocity;

impl Conversion for Velocity {
    type RegisterType = u16;
    type UsiType = f64;

    fn from_raw(raw: u16) -> f64 {
        use std::f64::consts::PI;
        let magnitude = (raw & 0x7FFF) as f64;
        let signed = if raw & 0x8000 != 0 {
            -magnitude
        } else {
            magnitude
        };
        (2.0 * PI * signed) / (4096.0 - 1.0)
    }

    fn to_raw(value: f64) -> Result<u16, Box<dyn Error>> {
        use std::f64::consts::PI;
        let raw_magnitude = (4096.0 - 1.0) * value.abs() / (2.0 * PI);
        if !raw_magnitude.is_finite() || raw_magnitude > 0x7FFF as f64 {
            return Err(Box::new(ConversionRangeError {
                what: "goal_speed",
                value: raw_magnitude as i64,
                min: -0x7FFF,
                max: 0x7FFF,
            }));
        }
        let mag = raw_magnitude as u16;
        Ok(if value < 0.0 { mag | 0x8000 } else { mag })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_decodes_low_12_bits() {
        assert_eq!(Position::from_raw(0x0000), 0);
        assert_eq!(Position::from_raw(0x0FFF), 4095);
        // BIT15 set is a direction flag, not part of the count.
        assert_eq!(Position::from_raw(0x8000), 0);
        assert_eq!(Position::from_raw(0x8123), 0x123);
        // Upper nibble bits above BIT11 should be masked off too.
        assert_eq!(Position::from_raw(0xF123), 0x123);
    }

    #[test]
    fn position_to_raw_rejects_out_of_range() {
        assert!(Position::to_raw(-1).is_err());
        assert!(Position::to_raw(4096).is_err());
        assert_eq!(Position::to_raw(0).unwrap(), 0);
        assert_eq!(Position::to_raw(4095).unwrap(), 4095);
    }

    #[test]
    fn offset_decodes_all_four_segments() {
        // Segment 1: 0..=2047 → 0..=+2047
        assert_eq!(Offset::from_raw(0), 0);
        assert_eq!(Offset::from_raw(2047), 2047);
        // Segment 2: 2048..=4095 → 0..=-2047
        assert_eq!(Offset::from_raw(2048), 0);
        assert_eq!(Offset::from_raw(4095), -2047);
        // Segment 3: 4096..=6143 → +2048..=+4095
        assert_eq!(Offset::from_raw(4096), 2048);
        assert_eq!(Offset::from_raw(6143), 4095);
        // Segment 4: 6144..=8191 → -2048..=-4095
        assert_eq!(Offset::from_raw(6144), -2048);
        assert_eq!(Offset::from_raw(8191), -4095);
    }

    #[test]
    fn offset_roundtrips() {
        for v in [-4095, -2048, -2047, -1, 0, 1, 2047, 2048, 4095] {
            let raw = Offset::to_raw(v).unwrap();
            assert_eq!(Offset::from_raw(raw), v, "round-trip failed for {v}");
        }
    }

    #[test]
    fn offset_to_raw_rejects_out_of_range() {
        assert!(Offset::to_raw(-4096).is_err());
        assert!(Offset::to_raw(4096).is_err());
    }

    #[test]
    fn signed_speed_decodes_bit15() {
        assert_eq!(SignedSpeed::from_raw(0), 0);
        assert_eq!(SignedSpeed::from_raw(0x7FFF), 32767);
        assert_eq!(SignedSpeed::from_raw(0x8000), 0);
        assert_eq!(SignedSpeed::from_raw(0xFFFF), -32767);
        assert_eq!(SignedSpeed::from_raw(0x8001), -1);
    }

    #[test]
    fn signed_speed_roundtrips() {
        for v in [-32767, -1, 0, 1, 32767] {
            let raw = SignedSpeed::to_raw(v).unwrap();
            assert_eq!(SignedSpeed::from_raw(raw), v);
        }
    }

    #[test]
    fn signed_speed_to_raw_rejects_out_of_range() {
        assert!(SignedSpeed::to_raw(-32768).is_err());
        assert!(SignedSpeed::to_raw(32768).is_err());
    }

    #[test]
    fn signed_load_decodes_bit10() {
        assert_eq!(SignedLoad::from_raw(0), 0);
        assert_eq!(SignedLoad::from_raw(0x03FF), 1023);
        assert_eq!(SignedLoad::from_raw(0x0400), 0);
        assert_eq!(SignedLoad::from_raw(0x07FF), -1023);
        assert_eq!(SignedLoad::from_raw(0x0401), -1);
    }

    #[test]
    fn signed_load_roundtrips() {
        for v in [-1023, -1, 0, 1, 1023] {
            let raw = SignedLoad::to_raw(v).unwrap();
            assert_eq!(SignedLoad::from_raw(raw), v);
        }
    }

    #[test]
    fn signed_load_to_raw_rejects_out_of_range() {
        assert!(SignedLoad::to_raw(-1024).is_err());
        assert!(SignedLoad::to_raw(1024).is_err());
    }
}
