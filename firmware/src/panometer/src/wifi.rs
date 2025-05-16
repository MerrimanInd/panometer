// Embassy Access Point module
// https://github.com/esp-rs/esp-hal/blob/esp-hal-v1.0.0-beta.0/examples/src/bin/wifi_embassy_access_point.rs
//! - creates an open access-point with SSID `esp-wifi`
//! - you can connect to it using a static IP in range 192.168.2.2 .. 192.168.2.255, gateway 192.168.2.1
//! - open http://192.168.2.1:8080/ in your browser - the example will perform an HTTP get request to some "random" server
//!
//! On Android you might need to choose _Keep Accesspoint_ when it tells you the WiFi has no internet connection, Chrome might not want to load the URL - you can use a shell and try `curl` and `ping`

use core::net::Ipv4Addr;
use core::str::FromStr;

use anyhow::anyhow;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_println as _;
use esp_println::println;
use esp_wifi::wifi::{self, WifiController, WifiDevice, WifiEvent, WifiState};
use esp_wifi::EspWifiController;

use crate::mk_static;

// const SSID: &str = env!("SSID");
// const PASSWORD: &str = env!("PASSWORD");
const SSID: &str = "espnet";
const PASSWORD: &str = "password";

// Unlike Station mode, You can give any IP range(private) that you like
// IP Address/Subnet mask eg: STATIC_IP=192.168.13.37/24
const STATIC_IP: &str = "192.168.1.2/24";
// Gateway IP eg: GATEWAY_IP="192.168.13.37"
const GATEWAY_IP: &str = "192.168.1.2";

pub async fn start_wifi(
    esp_wifi_ctrl: &'static EspWifiController<'static>,
    wifi: esp_hal::peripherals::WIFI,
    mut rng: Rng,
    spawner: &Spawner,
) -> anyhow::Result<Stack<'static>> {
    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, wifi).unwrap();
    let wifi_interface = interfaces.ap;
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    // Parse STATIC_IP
    let ip_addr =
        Ipv4Cidr::from_str(STATIC_IP).map_err(|_| anyhow!("Invalid STATIC_IP: {}", STATIC_IP))?;

    // Parse GATEWAY_IP
    let gateway = Ipv4Addr::from_str(GATEWAY_IP)
        .map_err(|_| anyhow!("Invalid GATEWAY_IP: {}", GATEWAY_IP))?;

    // Create Network config with IP details
    let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: ip_addr,
        gateway: Some(gateway),
        dns_servers: Default::default(),
    });

    // alternate approach
    // let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
    //     address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 2, 1), 24),
    //     gateway: Some(Ipv4Address::from_bytes(&[192, 168, 2, 1])),
    //     dns_servers: Default::default(),
    // });

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );

    spawner.spawn(connection_task(controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    spawner.spawn(dhcp_server(stack, GATEWAY_IP)).ok();

    wait_for_connection(stack).await;


    Ok(stack)
}

async fn wait_for_connection(stack: Stack<'_>) {
    println!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Connect to the AP `{SSID}` with the password `{PASSWORD}` and point your browser to http://{GATEWAY_IP}/");
    println!("DHCP is enabled so there's no need to configure a static IP, but just in case:");
    while !stack.is_config_up() {
        Timer::after(Duration::from_millis(100)).await
    }
    stack
        .config_v4()
        .inspect(|c| println!("ipv4 config: {c:?}"));
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::ApStarted => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(), // Set your password
                auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn dhcp_server(stack: Stack<'static>, gw_ip_addr: &'static str) {
    println!("starting dhcp task");

    use esp_hal_dhcp_server::{
        simple_leaser::{SimpleDhcpLeaser, SingleDhcpLeaser},
        structs::DhcpServerConfig,
        Ipv4Addr,
    };

    let ip = Ipv4Addr::from_str(gw_ip_addr).expect("dhcp task failed to parse gw ip");

    let config = DhcpServerConfig {
        ip: ip.clone(),
        lease_time: Duration::from_secs(3600),
        gateways: &[ip],
        subnet: None,
        dns: &[ip],
        use_captive_portal: true,
    };

    /*
    let mut leaser = SimpleDhcpLeaser {
        start: Ipv4Addr::new(192, 168, 2, 50),
        end: Ipv4Addr::new(192, 168, 2, 200),
        leases: Default::default(),
    };
    */
    let mut leaser = SingleDhcpLeaser::new(Ipv4Addr::new(192, 168, 1, 69));

    let res = esp_hal_dhcp_server::run_dhcp_server(stack, config, &mut leaser).await;
    if let Err(e) = res {
        println!("DHCP SERVER ERROR: {e:?}");
    }
}
