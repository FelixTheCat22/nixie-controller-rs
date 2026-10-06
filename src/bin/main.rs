#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // generator version: 1.4.0
    // generator parameters: -o esp32s3 -o unstable-hal -o embassy -o alloc -o wifi -o ble-trouble -o neovim-rustaceanvim -o defmt -o vscode -o probe-rs

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    info!("Embassy initialized!");

    let _wifi_controller =
        esp_radio::wifi::WifiController::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let _wifi_interface = esp_radio::wifi::Interface::station();
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32

    // TODO: Spawn some tasks
    spawner.spawn(nixie_controller_rs::ble::ble_task(peripherals.BT).unwrap());

    let tube_config = nixie_controller_rs::nixie_tubes::TubeConfig {
        latch_enable_pin: peripherals.GPIO18.into(),
        spi_clock_pin: peripherals.GPIO17.into(),
        spi_data_pin: peripherals.GPIO8.into(),
    };
    spawner.spawn(
        nixie_controller_rs::nixie_tubes::tube_driver_task(peripherals.SPI2.into(), tube_config)
            .unwrap(),
    );
}
