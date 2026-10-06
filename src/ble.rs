use bt_hci::controller::ExternalController;
use defmt::{info, warn};
use embassy_executor;
use embassy_futures::join::join;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::*;
use trouble_host::{HostResources, attribute::Uuid};

pub static TUBE_CONTROL_SIGNAL: Signal<CriticalSectionRawMutex, HeaplessString<8>> = Signal::new();

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

const DISPLAY_CONTROL_SERVICE_UUID: Uuid = uuid!("e2f9c75e-0fd0-4649-a3a2-1b132635ab4a");
const DISPLAY_CONTROL_VALUE_UUID: Uuid = uuid!("7798d68e-361a-4fe2-8817-a4f178ce149d");

#[gatt_server]
struct Server {
    tube_control: TubeControlService,
}

#[gatt_service(uuid=DISPLAY_CONTROL_SERVICE_UUID)]
struct TubeControlService {
    #[characteristic(uuid=DISPLAY_CONTROL_VALUE_UUID, read, write)]
    value: HeaplessString<8>,
}

#[embassy_executor::task]
pub async fn ble_task(bluetooth_peripheral: BT<'static>) {
    info!("Starting BLE task");
    let transport = BleConnector::new(bluetooth_peripheral, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 1>::new(transport);
    let mut resources: HostResources<_, DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(ble_controller, &mut resources).build();
    let mut runner = stack.runner();
    let mut periph = stack.peripheral();

    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "Nixie Clock",
        appearance: &appearance::CLOCK,
    }))
    .unwrap();

    join(
        async {
            loop {
                runner.run().await.unwrap()
            }
        },
        async {
            loop {
                let conn = advertise(&mut periph, &server).await;

                // Run GATT server
                let tcs_value = server.tube_control.value.clone();
                loop {
                    match conn.next().await {
                        GattConnectionEvent::Disconnected { reason } => {
                            info!("BLE device disconnected: {}", reason);
                            info!("going back to advertising.");
                            break;
                        }
                        GattConnectionEvent::Gatt { event } => {
                            match &event {
                                GattEvent::Write(w) => {
                                    if tcs_value.handle == w.handle() {
                                        TUBE_CONTROL_SIGNAL.signal(w.value(&tcs_value).unwrap());
                                    }
                                }
                                _ => {}
                            }
                            match event.accept() {
                                Ok(reply) => reply.send().await,
                                Err(e) => warn!("BLE GATT accept error: {:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
        },
    )
    .await;
}

async fn advertise<'stack, 'server, C, P, M>(
    periph: &mut Peripheral<'stack, C, P>,
    server: &'server AttributeServer<'_, M, P, _ATTRIBUTE_TABLE_SIZE, _CONNECTIONS_MAX>,
) -> GattConnection<'stack, 'server, P>
where
    C: Controller,
    P: PacketPool,
    M: RawMutex,
{
    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(b"Nixie Clock"),
        ],
        &mut adv_data[..],
    )
    .unwrap();
    let advertiser = periph
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[0..len],
                scan_data: &[],
            },
        )
        .await
        .unwrap();
    let conn = advertiser
        .accept()
        .await
        .unwrap()
        .with_attribute_server(server)
        .unwrap();
    info!("BLE device connected!");
    conn
}
