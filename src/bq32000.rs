use defmt::{info};
use esp_hal::{Blocking, gpio::AnyPin, i2c::master::{AnyI2c, Config as I2cConfig, ConfigError, I2c, Operation}, time::Rate};
use jiff::{Timestamp, Zoned, tz::TimeZone};

const DEVICE_ADDR: u8 = 0xD0;

const UTC: TimeZone = TimeZone::UTC;

struct Bq32000<'bq_driver> {
    i2c: I2c<'bq_driver, Blocking>
}

struct Bq32000Config<'bq_driver> {
    frequency: Rate,
    sda_pin: AnyPin<'bq_driver>,
    scl_pin: AnyPin<'bq_driver>
}

#[derive(defmt::Format)]
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
            Register::Minutes => 0x01,
            Register::Hours => 0x02,
            Register::Day => 0x03,
            Register::Date => 0x04,
            Register::Month => 0x05,
            Register::Year => 0x06,
        }
    }

    fn format(&self, current: u8, new: u8) -> u8 {
        match self {
            Register::Seconds |
            Register::Minutes => (current & 0x80) | (to_bcd(new) & 0x7F),
            Register::Hours => (current & 0xC0) | (to_bcd(new) & 0x3F),
            Register::Day => 0x0 | (new & 0x7),
            Register::Date => 0x0 | (to_bcd(new) & 0x3F),
            Register::Month => 0x0 | (to_bcd(new) & 0x1F),
            Register::Year => to_bcd(new),
        }
    }

    fn interpret(&self, value: u8) -> u8 {
        match self {
            Register::Seconds |
            Register::Minutes => from_bcd(value & 0x7F),
            Register::Hours => from_bcd(value & 0x3F),
            Register::Day => from_bcd(value & 0x07),
            Register::Date => from_bcd(value & 0x3F),
            Register::Month => from_bcd(value & 0x1F),
            Register::Year => from_bcd(value),
        }
    }
}

impl<'bq_driver> Bq32000<'bq_driver> {
    fn init(i2c_peripheral: AnyI2c<'bq_driver>, config: Bq32000Config<'bq_driver>) -> Result<Self, ConfigError> {
        info!("initializing bq32000 driver.");
        let i2c = I2c::new(
            i2c_peripheral,
            I2cConfig::default()
                .with_frequency(config.frequency)
        )?
        .with_scl(config.scl_pin)
        .with_sda(config.sda_pin);
        info!("bq32000 driver initialized.");

        Ok( Self { i2c } )
    }

    fn set_time(&mut self, time: Timestamp) {
        let time = time.to_zoned(UTC);
        self.write_register(&Register::Seconds, time.second() as u8);
        self.write_register(&Register::Minutes, time.minute() as u8);
        self.write_register(&Register::Hours, time.hour() as u8);
        self.write_register(&Register::Day, time.weekday().to_sunday_one_offset() as u8);
        self.write_register(&Register::Date, time.day() as u8);
        self.write_register(&Register::Month, time.month() as u8);
        self.write_register(&Register::Year, (time.year().abs() % 100) as u8);
        info!("set rtc chip time to {}.", time);
    }

    fn get_time(&mut self) -> Timestamp {
        let time = Zoned::default().with() // default is in UTC
            .second(self.read_register(&Register::Seconds, false) as i8)
            .minute(self.read_register(&Register::Minutes, false) as i8)
            .hour(self.read_register(&Register::Hours, false) as i8)
            .day(self.read_register(&Register::Date,false) as i8) // Register day contains weekday, use date
            .month(self.read_register(&Register::Month, false) as i8)
            .year(self.read_register(&Register::Year, false) as  i16 + 2000)
        .build().unwrap();
        info!("read time {} from rtc chip.", time);
        time.timestamp()
    }

    fn read_register(&mut self, reg: &Register, raw: bool) -> u8 {
        info!("reading register {}", reg);
        let mut buf: [u8; 1] = [0];
        self.i2c.transaction(DEVICE_ADDR, &mut [
            Operation::Write(&[reg.address()]),
            Operation::Read(&mut buf)
        ]).unwrap();
        if !raw {
            buf[0] = reg.interpret(buf[0]);
        }
        info!("read value {} from reg {}", buf[0], reg);
        return buf[0];
    }

    fn write_register(&mut self, reg: &Register, value: u8) {
        info!("writing value {} to register {}", value, reg);
        let current_value = self.read_register(reg, true);
        let new_value = reg.format(current_value, value);
        self.i2c.write(DEVICE_ADDR, &[new_value]).unwrap();
    }
}

// Helper functions
fn to_bcd(n: u8) -> u8 {
    ((n / 10) << 4) | (n % 10)
}

fn from_bcd(n: u8) -> u8 {
    let ones = n & 0x0F;
    let tens = (n & 0xF0) >> 4;
    (tens * 10) + ones
}