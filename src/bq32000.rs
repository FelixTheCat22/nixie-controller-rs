use defmt::{info, todo};
use esp_hal::{Blocking, gpio::AnyPin, i2c::master::{AnyI2c, Config as I2cConfig, ConfigError, I2c}, time::Rate};
use jiff::Timestamp;

const DEVICE_ADDR: u8 = 0xD0;

struct Bq32000<'bq_driver> {
    i2c: I2c<'bq_driver, Blocking>
}

struct Bq32000Config<'bq_driver> {
    frequency: Rate,
    sda_pin: AnyPin<'bq_driver>,
    scl_pin: AnyPin<'bq_driver>
}

enum Register {
    Seconds,
    Minutes,
    Hours,
    Day,
    Date,
    Month,
    Year,
}

impl Register {
    fn address(&self) -> u8 {
        match self {
            Register::Seconds => 0x00,
            Register::Minutes => 0x00,
            Register::Hours => 0x00,
            Register::Day => 0x00,
            Register::Date => 0x00,
            Register::Month => 0x00,
            Register::Year => 0x00,
        }
    }

    fn format(&self, current: u8, new: u8) -> u8 {
        match self {
            Register::Seconds |
            Register::Minutes => {
                return (current & 0x80) | (Self::bcd(new) & 0x7F)
            },
            Register::Hours => {
                return (current & 0xC0) | (Self::bcd(new) & 0x3F)
            },
            Register::Day => {
                return 0x0 | (new & 0x7)
            },
            Register::Date => {
                0x0 | (Self::bcd(new) & 0x3F)
            },
            Register::Month => {
                0x0 | (Self::bcd(new) & 0x1F)
            },
            Register::Year => {
                Self::bcd(new)
            },
        }
    }

    fn bcd(n: u8) -> u8 {
        return ((n / 10) << 4) | (n % 10)
    }
}

impl<'bq_driver> Bq32000<'bq_driver> {
    fn init(i2c_peripheral: AnyI2c<'bq_driver>, config: Bq32000Config<'bq_driver>) -> Result<Self, ConfigError> {
        info!("initializing bq32000 driver");
        let i2c = I2c::new(
            i2c_peripheral,
            I2cConfig::default()
                .with_frequency(config.frequency)
        )?
        .with_scl(config.scl_pin)
        .with_sda(config.sda_pin);
        info!("bq32000 driver initialized");

        Ok( Self { i2c } )
    }

    fn set_time(&mut self, time: Timestamp) {
        todo!()
    }

    fn get_time(&mut self) -> Timestamp {
        todo!()
    }

    fn read_register(&mut self, reg: Register) -> u8 {
        todo!()
    }

    fn write_register(&mut self, reg: Register, value: u8) {
        todo!()
    }
}