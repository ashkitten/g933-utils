//! A program to configure and control the Logitech G933 Gaming Headset

// Warn on missing documentation
#![warn(missing_docs)]
// Because otherwise clippy will warn us on use of format_err!, and I want to keep it consistent
#![cfg_attr(feature = "cargo-clippy", allow(useless_format))]

extern crate byteorder;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate udev;

#[macro_use]
mod macros;
pub mod battery;
pub mod buttons;
pub mod device_info;
pub mod lights;

use byteorder::{ByteOrder, BigEndian};
use failure::Error;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::str;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type RequestsMap = HashMap<[u8; 4], Sender<[u8; 20]>>;

/// Convert a struct that implements this trait to bytes
pub trait AsBytes {
    /// Convert a struct that implements this trait to bytes
    fn as_bytes(&self) -> Vec<u8>;
}

/// Convert a series of bytes to a struct that implements this trait
pub trait FromBytes {
    /// Convert a series of bytes to a struct that implements this trait
    fn from_bytes(bytes: &[u8]) -> Self;
}

/// Contains a `HidDevice` and a vector of requests to be processed
pub struct Device {
    file: File,
    requests: Arc<Mutex<RequestsMap>>,
}

impl Device {
    /// Construct a new `Device` from a `HidDevice`
    pub fn new(path: &Path) -> Result<Self, Error> {
        let device = Self {
            file: OpenOptions::new().read(true).write(true).open(path)?,
            requests: Arc::new(Mutex::new(HashMap::new())),
        };

        let mut file = device.file.try_clone()?;
        let requests = Arc::clone(&device.requests);
        thread::spawn(move || {
            use std::io::Read;

            let mut data = [0u8; 20];

            loop {
                thread::sleep(Duration::from_millis(100));

                let mut requests = requests.lock().unwrap();

                // If there are no requests or it times out without reading anything, loop again
                if requests.is_empty() || file.read(&mut data).unwrap() == 0 {
                    continue;
                }

                if let Some(sender) = requests.remove(&data[..4]) {
                    debug!(
                        "Got data from device: {}",
                        data.iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<String>>()
                            .join(" ")
                    );
                    sender.send(data).unwrap();
                }
            }
        });

        Ok(device)
    }

    /// Send a raw request to the device
    pub fn raw_request(&mut self, request: &[u8]) -> Result<[u8; 20], Error> {
        use std::io::Write;

        ensure!(request.len() <= 20, "Request is longer than 20 bytes");

        let mut data = [0u8; 20];
        data[..request.len()].copy_from_slice(request);

        // Block until no similar requests are pending
        loop {
            let requests = self.requests.lock().unwrap();
            if !requests.contains_key(&data[..4]) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        let (sender, receiver) = mpsc::channel();

        // Make sure we drop the lock before our write/read loop
        {
            let mut requests = self.requests.lock().unwrap();

            let mut header = [0u8; 4];
            header.copy_from_slice(&data[..4]);
            requests.insert(header, sender);
        }

        // Try 3 times then fail if it doesn't return anything
        for _ in 0..3 {
            self.file.write_all(&data)?;
            debug!(
                "Sent data to device: {}",
                data.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ")
            );
            match receiver.recv_timeout(Duration::from_secs(2)) {
                Ok(response) => return Ok(response),
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(error) => return Err(error.into()),
            }
        }

        bail!("Request timed out");
    }

    /// Get info about a feature
    pub fn get_feature(&mut self, feature: u16) -> Result<(u8, u8, u8), Error> {
        let mut feature_bytes = [0; 2];
        BigEndian::write_u16(&mut feature_bytes, feature);
        let request = v![0x11, 0xff, 0x00, 0x01, @feature_bytes.iter()];
        self.raw_request(&request)
            .map(|response| (response[4], response[5], response[6]))
    }

    /// Get protocol version of device
    pub fn get_protocol_version(&mut self) -> Result<(u8, u8), Error> {
        let request = [0x11, 0xff, 0x00, 0x11, 0x00, 0x00, 0xaf];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[6] == 0xaf,
                    "Ping response did not match the request: was {}",
                    response[6],
                );
                Ok((response[4], response[5]))
            }
            Err(error) => Err(error),
        }
    }

    /// Get device info
    pub fn get_device_info(&mut self) -> Result<device_info::DeviceInfo, Error> {
        let request = [0x11, 0xff, 0x02, 0x01];
        self.raw_request(&request)
            .map(|response| device_info::DeviceInfo::from_bytes(&response[4..]))
    }

    /// Get device name
    pub fn get_device_name(&mut self) -> Result<String, Error> {
        let request = [0x11, 0xff, 0x03, 0x01];
        let length = self.raw_request(&request)?[4];

        let mut name = String::new();
        for i in 0..length / 10 {
            let request = [0x11, 0xff, 0x03, 0x11, i];
            let response = &self.raw_request(&request)?[4..20]; // blaze it
                                                                // Safe, probably
            name += str::from_utf8(response).unwrap();
        }

        // Trim null characters off the end
        name = name.trim_right_matches('\0').to_string();

        Ok(name)
    }

    /// Set light configuration
    pub fn set_lights(&mut self, lights: &lights::Config) -> Result<lights::Config, Error> {
        let request = v![0x11, 0xff, 0x04, 0x31, @lights.as_bytes()];
        Ok(lights::Config::from_bytes(&self.raw_request(&request)?[4..]))
    }

    /// Get startup effect enabled status
    pub fn get_startup_effect_enabled(&mut self) -> Result<bool, Error> {
        let request = [0x11, 0xff, 0x04, 0x41, 0x00, 0x01];
        self.raw_request(&request)
            .map(|response| response[4] == 0x01)
    }

    /// Set startup effect on or off
    pub fn enable_startup_effect(&mut self, enable: bool) -> Result<(), Error> {
        let enable_byte = if enable { 0x01 } else { 0x02 };
        let request = [0x11, 0xff, 0x04, 0x51, 0x00, 0x01, enable_byte];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[6] == enable_byte,
                    "enable_startup_effect response did not match the request: expected {}, was {}",
                    enable_byte,
                    response[6],
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Get number of buttons on device
    pub fn get_button_count(&mut self) -> Result<u8, Error> {
        let request = [0x11, 0xff, 0x05, 0x01];
        self.raw_request(&request).map(|response| response[4])
    }

    /// Get button reporting status
    pub fn get_buttons_enabled(&mut self) -> Result<bool, Error> {
        let request = [0x11, 0xff, 0x05, 0x11];
        self.raw_request(&request)
            .map(|response| response[4] == 0x01)
    }

    /// Set button reporting on or off
    pub fn enable_buttons(&mut self, enable: bool) -> Result<(), Error> {
        let request = [0x11, 0xff, 0x05, 0x21, enable as u8];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[4] == enable as u8,
                    "enable_buttons response did not match the request: expected {}, was {}",
                    enable as u8,
                    response[4],
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Get frequency of equalizer bands
    pub fn get_equalizer_bands(&mut self) -> Result<[u16; 10], Error> {
        let mut bands = [0; 10];
        let request = [0x11, 0xff, 0x06, 0x11, 0x00];
        BigEndian::read_u16_into(&self.raw_request(&request)?[5..19], &mut bands[0..7]);
        let request = [0x11, 0xff, 0x06, 0x11, 0x07];
        BigEndian::read_u16_into(&self.raw_request(&request)?[5..11], &mut bands[7..10]);
        Ok(bands)
    }

    /// Get equalizer
    pub fn get_equalizer(&mut self) -> Result<[i8; 10], Error> {
        let request = [0x11, 0xff, 0x06, 0x21];
        self.raw_request(&request).map(|response| unsafe {
            let mut config = [0; 10];
            config.copy_from_slice(&response[4..14]);
            std::mem::transmute(config)
        })
    }

    /// Set equalizer
    pub fn set_equalizer(&mut self, permanent: bool, config: [i8; 10]) -> Result<(), Error> {
        let permanent = if permanent { 0x02 } else { 0x00 };
        let config = unsafe { std::mem::transmute::<_, [u8; 10]>(config) };
        // TODO: figure out what the 0x02 is
        let request = v![0x11, 0xff, 0x06, 0x31, permanent, @config.iter()];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[5..15] == config,
                    "set_equalizer response did not match the request: expected {:?}, was {:?}",
                    config,
                    &response[5..15]
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Get sidetone volume
    pub fn get_sidetone_volume(&mut self) -> Result<u8, Error> {
        let request = [0x11, 0xff, 0x07, 0x00];
        self.raw_request(&request).map(|response| response[4])
    }

    /// Set sidetone volume
    pub fn set_sidetone_volume(&mut self, volume: u8) -> Result<(), Error> {
        let request = [0x11, 0xff, 0x07, 0x11, volume];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[4] == volume,
                    "set_sidetone_volume response did not match request: expected {}, was {}",
                    volume,
                    response[4],
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Get battery status and level
    pub fn get_battery_status(&mut self) -> Result<battery::BatteryStatus, Error> {
        let request = [0x11, 0xff, 0x08, 0x01];
        Ok(battery::BatteryStatus::from_bytes(
            &self.raw_request(&request)?[4..],
        ))
    }

    /// Get poweroff timeout
    pub fn get_poweroff_timeout(&mut self) -> Result<Option<u8>, Error> {
        let request = [0x11, 0xff, 0x08, 0x11];
        match self.raw_request(&request)?[4] {
            0 => Ok(None),
            t => Ok(Some(t)),
        }
    }

    /// Set poweroff timeout
    pub fn set_poweroff_timeout(&mut self, timeout: Option<u8>) -> Result<(), Error> {
        let timeout = match timeout {
            None => 0,
            Some(t) => t,
        };
        let request = [0x11, 0xff, 0x08, 0x21, timeout];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[4] == timeout,
                    "set_poweroff_timeout response did not match request: expected {}, was {}",
                    timeout,
                    response[4],
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Watch for button presses/releases (g1, g2, g3)
    pub fn watch_buttons(&mut self, callback: fn(buttons::Buttons)) -> Result<(), Error> {
        let (sender, receiver) = mpsc::channel();

        // Loop and keep adding the request to our pending request map
        loop {
            // Make sure we drop the lock before we try reading
            {
                let mut requests = self.requests.lock().unwrap();

                let header = [0x11, 0xff, 0x05, 0x00];
                requests.insert(header, sender.clone());
            }

            match receiver.recv_timeout(Duration::from_secs(2)) {
                Ok(response) => callback(buttons::Buttons::from_bytes(&response[4..])),
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

/// Enumerate and initialize devices
pub fn find_devices() -> Result<HashMap<String, Device>, Error> {
    let context = udev::Context::new()?;

    let mut enumerator = udev::Enumerator::new(&context)?;
    enumerator.match_subsystem("usb")?;
    enumerator.match_attribute("idVendor", "046d")?;
    enumerator.match_attribute("idProduct", "0a5b")?;
    let parents = enumerator.scan_devices()?;

    let mut devices = HashMap::new();
    for parent in parents {
        info!("Found usb device: {}", parent.sysname().to_str().unwrap());

        let mut enumerator = udev::Enumerator::new(&context)?;
        enumerator.match_subsystem("hidraw")?;
        enumerator.match_parent(&parent)?;
        devices.insert(
            parent.sysname().to_string_lossy().to_string(),
            Device::new(enumerator
                .scan_devices()?
                .next()
                .ok_or_else(|| format_err!("Parent does not contain any hidraw devices"))?
                .devnode()
                .ok_or_else(|| format_err!("Hidraw device does not have a filesystem node"))?)?,
        );
    }

    Ok(devices)
}
