use defmt::info;
use esp_hal::{
    Blocking, gpio::{AnyPin, Level, Output, OutputConfig}, spi::{Error as SpiError, Mode, master::{AnySpi, Config as SpiConfig, ConfigError, Spi}
    }, time::Rate
};
use trouble_host::prelude::HeaplessString;

use crate::ble::TUBE_CONTROL_SIGNAL as BLE_CONTROL_SIGNAL;

#[derive(defmt::Format)]
struct NixieTubes<'tube_driver> {
    latch_enable: Output<'tube_driver>,
    spi: Spi<'tube_driver, Blocking>
}

pub struct TubeConfig<'tube_driver> {
    pub latch_enable_pin: AnyPin<'tube_driver>,
    pub spi_clock_pin: AnyPin<'tube_driver>,
    pub spi_data_pin: AnyPin<'tube_driver>
}

#[repr(u16)]
#[derive(Clone, Copy, defmt::Format)]
enum NixieDigit {
    LDP    = !(1 << 11) & 0xFFF,
    Digit1 = !(1 << 10) & 0xFFF,
    Digit2 = !(1 << 9)  & 0xFFF,
    Digit3 = !(1 << 8)  & 0xFFF,
    Digit4 = !(1 << 7)  & 0xFFF,
    Digit5 = !(1 << 6)  & 0xFFF,
    Digit6 = !(1 << 5)  & 0xFFF,
    Digit7 = !(1 << 4)  & 0xFFF,
    Digit8 = !(1 << 3)  & 0xFFF,
    Digit9 = !(1 << 2)  & 0xFFF,
    Digit0 = !(1 << 1)  & 0xFFF,
    RDP    = !1,
    Off    = !0
}

impl NixieDigit {
    fn from_char(digit: char) -> NixieDigit {
        match digit {
            '.' => NixieDigit::LDP,
            '1' => NixieDigit::Digit1,
            '2' => NixieDigit::Digit2,
            '3' => NixieDigit::Digit3,
            '4' => NixieDigit::Digit4,
            '5' => NixieDigit::Digit5,
            '6' => NixieDigit::Digit6,
            '7' => NixieDigit::Digit7,
            '8' => NixieDigit::Digit8,
            '9' => NixieDigit::Digit9,
            '0' => NixieDigit::Digit0,
            ',' => NixieDigit::RDP,
            _ => NixieDigit::Off,
        }
    }

    fn parse_str(digits: &str) -> [NixieDigit; 8] {
        let mut ret = [NixieDigit::Off; 8];
        for (i, digit) in digits.char_indices() {
            if i > 7 { return ret; }
            ret[i] = Self::from_char(digit);
        }
        ret
    }
}

fn pack_digits(digits: [NixieDigit; 8]) -> [u8; 12] {
    let mut ret_idx = 0;
    let mut ret: [u8; 12] = [0; 12];
    for i in (0..8).step_by(2) {
        let digit0: u16 = digits[i] as u16;
        let digit1: u16 = digits[i+1] as u16;

        ret[ret_idx] = ((digit0 & 0x0FF0) >> 4) as u8;
        ret_idx += 1;
        ret[ret_idx] = (((digit0 & 0x000F) << 4) |
                        ((digit1 & 0x0F00) >> 8)) as u8;
        ret_idx += 1;
        ret[ret_idx] = (digit1 & 0x00FF) as u8;
        ret_idx += 1;
    }
    ret
}

impl<'tube_driver> NixieTubes<'tube_driver> {
    fn init(spi_peripheral: AnySpi<'tube_driver>, config: TubeConfig<'tube_driver>) -> Result<Self, ConfigError> {
        info!("Initializing nixie tube driver");
        // Latch enable GPIO
        let gpio_config = OutputConfig::default();
        let latch_enable = Output::new(config.latch_enable_pin, Level::High, gpio_config);
        info!("Initialized latch enable pin.");

        info!("Initializing SPI bus");
        let spi = Spi::new(
            spi_peripheral,
            SpiConfig::default()
                .with_frequency(Rate::from_hz(78125)) // Minimum value, thank god it works
                .with_mode(Mode::_3)
        )?
        .with_mosi(config.spi_data_pin)
        .with_sck(config.spi_clock_pin);
        info!("Initialized SPI bus");

        return Ok( Self { latch_enable, spi } );
    }

    fn write<T: TubeWriteable>(&mut self, value: T) -> Result<(), SpiError> {
        info!("writing to tubes: {:?}", value);
        let bits = value.to_bits();
        self.latch_enable.toggle();
        self.spi.write(&bits)?;
        self.latch_enable.toggle();
        Ok(())
    }
}

trait TubeWriteable: defmt::Format {
    fn to_bits(&self) -> [u8; 12];
}

impl TubeWriteable for &str {
    fn to_bits(&self) -> [u8; 12] {
        let digits = NixieDigit::parse_str(self);
        pack_digits(digits)
    }
}

impl TubeWriteable for HeaplessString<8> {
    fn to_bits(&self) -> [u8; 12] {
        let s: &str = &self;
        s.to_bits()
    } 
}

impl TubeWriteable for [NixieDigit; 8] {
    fn to_bits(&self) -> [u8; 12] {
        pack_digits(*self)
    }
}

impl TubeWriteable for [u8; 12] {
    fn to_bits(&self) -> [u8; 12] {
        *self
    }
}

#[embassy_executor::task]
pub async fn tube_driver_task(spi_peripheral: AnySpi<'static>, config: TubeConfig<'static>) {
    let mut tubes = NixieTubes::init(spi_peripheral, config).unwrap();
    loop {
        let value = BLE_CONTROL_SIGNAL.wait().await;
        tubes.write(value).unwrap();
    }
}