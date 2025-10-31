use anyhow::{self, Ok};
use embedded_graphics::{
  mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
  pixelcolor::BinaryColor,
  prelude::*,
  text::{Baseline, Text},
};
use embedded_svc::http::Method;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{
  Configuration as HttpServerConfig, EspHttpServer,
};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use std::cell::UnsafeCell;
use std::{thread::sleep, time::Duration};
mod constants;
use constants::{PASSWORD, SSID};

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

  // Initialize Display Over I2C
  let mut display = {
    let config = I2cConfig::new().baudrate(100.kHz().into());
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let i2c =
      esp_idf_hal::i2c::I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;
    let interface = I2CDisplayInterface::new(i2c);
    Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
      .into_buffered_graphics_mode()
  };

  let text_style = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

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
  display.init().unwrap();
  Text::with_baseline(
    "Connecting to ",
    Point::new(30, 20),
    text_style,
    Baseline::Top,
  ).draw(&mut display).unwrap();
  Text::with_baseline(
    "Wifi...",
    Point::new(30, 32),
    text_style,
    Baseline::Top,
  )
  .draw(&mut display)
  .unwrap();
  display.flush().unwrap();
  wifi.connect()?;

  wifi.wait_netif_up()?;

  log::info!("Connected to WiFi!");
  display.clear(BinaryColor::Off).unwrap();
  Text::with_baseline(
    "Connected to ",
    Point::new(30, 20),
    text_style,
    Baseline::Top,

  )
  .draw(&mut display)
  .unwrap();
  Text::with_baseline(
    format!("SSID: {:?}", SSID).as_str(),
    Point::new(30, 32),
    text_style,
    Baseline::Top,
  )
  .draw(&mut display)
  .unwrap();
  display.flush().unwrap();

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

  httpserver.fn_handler(
    "/",
    Method::Get,
    |request| -> Result<(), anyhow::Error> {
      let html = index_html();
      let mut response = request.into_ok_response()?;
      response.write(html.as_bytes())?;
      Ok(())
    },
  )?;

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
    <body style="text-align: center;">
    <h1>Welcome to the ESP32 Web Server!</h1>
    <br/>
    <h1>Control the LED</h1>
    <h2>Go to <a href="/change">/change</a> to toggle the LED.</h2>
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
    <body style="text-align: center;">
    <h1 style="font-size: 32px">LED toggled!</h1>
    <h1>To toggle again, press the button.</h1>
    <br/>
    <button onclick="location.href='/change'" style="padding: 10px 20px; font-size: 32px;">Toggle</button>
    </body>
</html>
"#
  )
}
