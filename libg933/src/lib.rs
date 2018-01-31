//! A program to configure and control the Logitech G933 Gaming Headset

#![feature(drain_filter)]
#![warn(missing_docs)]

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
pub mod lights;

use failure::Error;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    requests: Arc<Mutex<HashMap<[u8; 4], Sender<[u8; 20]>>>>,
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
                    debug!("Got data from device: {:?}", data);
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

        loop {
            self.file.write(&data)?;
            debug!("Sent data to device: {:?}", data);
            match receiver.recv_timeout(Duration::from_secs(2)) {
                Ok(response) => return Ok(response),
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Get info about a feature
    pub fn get_feature(&mut self, feature: u16) -> Result<(u8, u8, u8), Error> {
        let request = [
            0x11,
            0xff,
            0x00,
            0x01,
            (feature >> 8) as u8,
            (feature & 0xff) as u8,
        ];
        self.raw_request(&request)
            .map(|response| (response[4], response[5], response[6]))
    }

    /// Get protocol version of device
    pub fn get_protocol_version(&mut self) -> Result<(u8, u8), Error> {
        let request = [0x11, 0xff, 0x00, 0x11, 0x00, 0x00, 0xaf];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[2] == 0xaf,
                    "Ping response did not match the request: was {}",
                    response[2]
                );
                Ok((response[4], response[5]))
            }
            Err(error) => Err(error),
        }
    }

    /// Get device info
    pub fn get_device_info(&mut self) -> Result<(u8, [u8; 4], [u8; 2], [u8; 6]), Error> {
        let request = [0x11, 0xff, 0x02, 0x01];
        self.raw_request(&request).map(|response| {
            let entity_cnt = response[4];
            let mut unit_id = [0; 4];
            unit_id.copy_from_slice(&response[5..9]);
            let mut transport = [0; 2];
            transport.copy_from_slice(&response[9..11]);
            let mut model_id = [0; 6];
            model_id.copy_from_slice(&response[11..17]);
            (entity_cnt, unit_id, transport, model_id)
        })
    }

    /// Set button reporting on or off
    pub fn set_report_buttons(&mut self, report_buttons: bool) -> Result<(), Error> {
        let request = [0x11, 0xff, 0x05, 0x21, report_buttons as u8];
        match self.raw_request(&request) {
            Ok(response) => {
                ensure!(
                    response[4] == report_buttons as u8,
                    "set_report_buttons response did not match the request: expected {}, was {}",
                    report_buttons as u8,
                    response[4]
                );
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Set light configuration
    pub fn set_lights(&mut self, lights: lights::Config) -> Result<lights::Config, Error> {
        let request = v![0x11, 0xff, 0x04, 0x31, @lights.as_bytes()];
        Ok(lights::Config::from_bytes(&self.raw_request(&request)?))
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
                    response[4]
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
            &self.raw_request(&request)?,
        ))
    }

    /// Set a listener for button presses/releases (g1, g2, g3)
    pub fn listen_buttons(&mut self, callback: fn(buttons::Buttons)) {
        // TODO: implement
        unimplemented!();
    }
}

/// Enumerate and initialize devices
pub fn find_devices() -> Result<Vec<Device>, Error> {
    let context = udev::Context::new()?;

    let mut enumerator = udev::Enumerator::new(&context)?;
    enumerator.match_subsystem("usb")?;
    enumerator.match_attribute("idVendor", "046d")?;
    enumerator.match_attribute("idProduct", "0a5b")?;
    Ok(enumerator
        .scan_devices()?
        .map(|parent| -> Result<_, Error> {
            let mut enumerator = udev::Enumerator::new(&context)?;
            enumerator.match_subsystem("hidraw")?;
            enumerator.match_parent(&parent)?;
            Ok(enumerator.scan_devices()?.filter_map(|device| {
                if let Some(devnode) = device.devnode() {
                    Some(Device::new(devnode).unwrap())
                } else {
                    None
                }
            }))
        })
        .filter_map(|result| {
            if let Ok(device) = result {
                Some(device)
            } else {
                error!("{}", result.err().unwrap());
                None
            }
        })
        .flat_map(|devices| devices)
        .collect())
}
