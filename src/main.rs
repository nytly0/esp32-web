use anyhow;
use embedded_svc::http::Method;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{Configuration as HttpServerConfig, EspHttpServer};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use std::cell::UnsafeCell;
use std::{thread::sleep, time::Duration};

const SSID: &str = "";
const PASSWORD: &str = "";

fn initialize() {
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Initialization complete!");
}
fn main() -> anyhow::Result<()> {
    initialize();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let led = UnsafeCell::new(esp_idf_hal::gpio::PinDriver::output(
        peripherals.pins.gpio18,
    )?);
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::None,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    println!("Wifi Connected, Starting HTTP Server");

    let mut httpserver = EspHttpServer::new(&HttpServerConfig::default())?;
    httpserver.fn_handler(
        "/change",
        Method::Get,
        move |request| -> Result<(), anyhow::Error> {
            let html = change_html();
            let mut response = request.into_ok_response()?;
            response.write(html.as_bytes())?;
            let led = unsafe { &mut *led.get() };
            led.toggle()?;
            Ok(())
        },
    )?;

    httpserver.fn_handler("/", Method::Get, |request| -> Result<(), anyhow::Error> {
        let html = index_html();
        let mut response = request.into_ok_response()?;
        response.write(html.as_bytes())?;
        Ok(())
    })?;

    // Loop to Avoid Program Termination
    loop {
        sleep(Duration::from_millis(1000));
    }
}

fn index_html() -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>Esp32 Web Server</title>
    </head>
    <body>
    <h1>Go to <a href="/change">/change</a> to toggle the LED.</h1>
    </body>
</html>
"#
    )
}

fn change_html() -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>Esp32 Web Server</title>
    </head>
    <body>
    <h1>LED toggled! To toggle again, go to <a href="/change">/change</a>.</h1>
    </body>
</html>
"#
    )
}
