use std::error::Error;
use std::fmt;

pub trait Conversion {
    type RegisterType;
    type UsiType;
    fn from_raw(raw: Self::RegisterType) -> Self::UsiType;
    fn to_raw(value: Self::UsiType) -> Result<Self::RegisterType, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct ConversionRangeError {
    pub what: &'static str,
    pub value: i64,
    pub min: i64,
    pub max: i64,
}

impl fmt::Display for ConversionRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} value {} out of range [{}, {}]",
            self.what, self.value, self.min, self.max
        )
    }
}

impl Error for ConversionRangeError {}

impl Conversion for bool {
    type RegisterType = u8;
    type UsiType = bool;

    fn from_raw(raw: u8) -> bool {
        raw != 0
    }

    fn to_raw(value: bool) -> Result<u8, Box<dyn Error>> {
        Ok(if value { 1 } else { 0 })
    }
}
