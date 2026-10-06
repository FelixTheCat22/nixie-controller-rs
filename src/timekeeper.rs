use embassy_futures::join::join;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
use embassy_time::{Duration, Ticker};
use esp_hal::{peripherals::RTC_TIMER, rtc_cntl::Rtc};
use jiff::Timestamp;

static TIME_MUTEX: Mutex<CriticalSectionRawMutex, Timestamp> = Mutex::new(Timestamp::UNIX_EPOCH);
static SET_TIME_SIGNAL: Signal<CriticalSectionRawMutex, Timestamp> = Signal::new();

#[embassy_executor::task]
async fn timekeeper_task(peripheral_rtc: RTC_TIMER<'static>) {
    let mut ticker = Ticker::every(Duration::from_millis(500));
    let rtc = Rtc::new(peripheral_rtc);

    let publish = async {
        loop {
            publish_time(&rtc).await;
            ticker.next().await;
        }
    };

    let set = async {
        loop {
            let new_timestamp = SET_TIME_SIGNAL.wait().await;
            rtc.set_current_time_us(new_timestamp.as_microsecond() as u64);
            publish_time(&rtc).await;
        }
    };

    join(publish, set).await;
}

async fn publish_time(rtc: &Rtc<'_>) {
    let timestamp = Timestamp::from_microsecond(rtc.current_time_us() as i64).unwrap();
    let mut time_mutex = TIME_MUTEX.lock().await;
    *time_mutex = timestamp;
    drop(time_mutex);
}
