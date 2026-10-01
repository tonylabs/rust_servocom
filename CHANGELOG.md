# Changelog

## 1.2.0 — 2026-10-02

### Added

- `HD1910` Feetech servo (`servo::feetech::hd1910`, `Hd1910PyController`), model number 7946. The register map is the STS3215's; `offset` (31), `mode` (33) and 34–39 are not yet verified against an HD1910 control table.
- Dynamixel protocol 2.0 Fast Sync Read (instruction 0x8A): `DynamixelProtocolHandler::fast_sync_read`, `with_fast_sync_read` / `set_fast_sync_read` / `fast_sync_read_enabled` on the handler, and `with_fast_sync_read` / `set_fast_sync_read` on every v2 controller (Python: `set_fast_sync_read`). When enabled, `sync_read*` sends one instruction and parses the single combined status packet, checking every motor's running CRC. Needs firmware that implements it (XL330: v46+).

### Fixed

- Dynamixel protocol 2.0 byte stuffing. Instruction packets whose parameters contain `FF FF FD` are now stuffed, and status packets are de-stuffed after the CRC check. Previously a read whose data happened to contain that pattern (e.g. a present current of -1 followed by a 0xFD byte) was mis-parsed.

## 1.1.0 — 2026-05-14

### Breaking

`read_present_position`, `read_offset`, `read_min/max_angle_limit`,
`read_present_speed`, `read_present_load` (and their sync variants) now return
decoded `i32` values in documented physical ranges. The previous raw-byte
behavior is available unchanged via the `read_raw_*` methods.

Affected controllers: `Sts3215Controller`, `Sts3025BlController`,
`Sm40BlController` (and their `*PyController` wrappers). The older `Scs0009`
family is unchanged — it uses a different (300°, 10-bit, big-endian)
encoding that does not fit the STS/SM control table.

Decoded ranges and units:

- `read_present_position` → `i32` in `[0, 4095]` (12-bit single-turn encoder count, 0°–360°). BIT15 of the raw register is the direction flag and is masked off.
- `read_offset`, `read_min_angle_limit`, `read_max_angle_limit` → `i32` in `[-4095, +4095]` (FeeTech 4-segment sign-magnitude encoding). Writes outside this range now return an error rather than silently wrapping.
- `read_present_speed` → `i32`, sign-magnitude via BIT15 of the raw register. Kept in native register units (0.0146 RPM or 0.732 RPM per step depending on the `phase` register).
- `read_present_load` → `i32` in `[-1023, +1023]` (0.1% duty cycle steps; sign-magnitude via BIT10 of the raw register, *not* BIT15).

`read_raw_*` / `write_raw_*` are byte-for-byte identical to the on-the-wire
register value (now typed `u16` everywhere instead of a mix of `i16`/`u16`).

The `Conversion` trait's `to_raw` method now returns
`Result<Self::RegisterType, Box<dyn Error>>` so range-checked encoders can
signal out-of-range inputs without panicking or clamping. Custom `Conversion`
impls outside this crate must be updated to match the new signature.

### Notes for reviewers

The following STS/SM registers still use the previous `AnglePosition` (radians
via the Dynamixel 4096/2π formula) and were *not* touched, since the spec only
called out the five read registers above:

- `goal_position` (0x2A) on Sts3215 / Sts3025Bl / Sm40Bl
- `goal_speed` (0x2E) on Sts3215 / Sts3025Bl / Sm40Bl (uses `Velocity`)

If you want goal_position decoded the same way as present_position (12-bit
encoder count), or goal_speed in the same signed native units as
present_speed, those are follow-up changes — they would also be breaking.
