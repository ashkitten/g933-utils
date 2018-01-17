//! # Features
//!
//! - 0x0000: root feature ([documentation][0000])
//!   - index: 0x00
//! - 0x0001: ????
//!   - index: 0x01
//! - 0x0003 device information ([documentation][0003])
//!   - index: 0x02
//! - 0x0005: ????
//!   - index: 0x03
//! - 0x1f20: ????
//!   - index: 0x08
//! - 0x8010: ????
//!   - index: 0x05
//! - 0x8070: ????
//!   - index: 0x04
//! - 0x8300: ????
//!   - index: 0x07
//! - 0x8310: ????
//!   - index: 0x06
//!
//! [0000]: https://lekensteyn.nl/files/logitech/x0000_root.html
//! [0003]: https://lekensteyn.nl/files/logitech/x0003_deviceinfo.html

#![feature(drain_filter, conservative_impl_trait)]
#![warn(missing_docs)]

extern crate futures;
extern crate udev;

pub mod light_config;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::collections::HashMap;
use futures::prelude::*;
use futures::sync::oneshot;

use light_config::*;

/// Convert a struct that implements this trait to bytes
pub trait AsBytes {
    /// Convert a struct that implements this trait to bytes
    fn as_bytes(&self) -> Vec<u8>;
}

/// Contains a `HidDevice` and a vector of requests to be processed
pub struct Device {
    file: File,
    requests: Arc<Mutex<HashMap<[u8; 4], oneshot::Sender<[u8; 16]>>>>,
}

impl Device {
    /// Construct a new `Device` from a `HidDevice`
    pub fn new(path: &Path) -> Self {
        let device = Self {
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap(),
            requests: Arc::new(Mutex::new(HashMap::new())),
        };

        let mut file = device.file.try_clone().unwrap();
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
                    let mut response = [0u8; 16];
                    response.copy_from_slice(&data[4..]);
                    sender.send(response).unwrap();
                }
            }
        });

        device
    }

    /// Send a raw request to the device
    pub fn raw_request(
        &mut self,
        request: &[u8],
    ) -> impl Future<Item = [u8; 16], Error = oneshot::Canceled> {
        assert!(request.len() <= 20);

        use std::io::Write;

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

        let mut requests = self.requests.lock().unwrap();
        let (sender, receiver) = oneshot::channel();

        let mut header = [0u8; 4];
        header.copy_from_slice(&data[..4]);
        requests.insert(header, sender);

        // TODO: re-send request if timeout
        self.file.write(&data).unwrap();

        Box::new(receiver)
    }

    /// getFeature ([documentation][doc])
    ///
    /// # Parameters:
    ///
    /// - `featId`
    ///
    /// # Return values:
    ///
    /// - `featIndex`
    /// - `featType`
    /// - `featVer`
    ///
    /// [doc]: https://lekensteyn.nl/files/logitech/x0000_root.html#getProtocolVersion
    pub fn get_feature(
        &mut self,
        feature: u16,
    ) -> impl Future<Item = (u8, u8, u8), Error = oneshot::Canceled> {
        let request = [
            0x11,
            0xff,
            0x00,
            0x01,
            (feature >> 8) as u8,
            (feature & 0xff) as u8,
        ];
        self.raw_request(&request)
            .map(|response| (response[0], response[1], response[2]))
    }

    /// getProtocolVersion ([documentation][doc])
    ///
    /// # Parameters:
    ///
    /// - `zero: u8` (padding)
    /// - `zero: u8` (padding)
    /// - `pingData: u8` (random byte)
    ///
    /// # Return values:
    ///
    /// - `protocolNum`
    /// - `targetSw`
    ///
    /// [doc]: https://lekensteyn.nl/files/logitech/x0000_root.html#getProtocolVersion
    pub fn get_protocol_version(
        &mut self,
    ) -> impl Future<Item = (u8, u8), Error = oneshot::Canceled> {
        let request = [0x11, 0xff, 0x00, 0x11, 0x00, 0x00, 0xee];
        self.raw_request(&request).map(|response| {
            assert_eq!(0xee, response[2]);
            (response[0], response[1])
        })
    }

    /// getDeviceInfo ([documentation][doc])
    ///
    /// # Parameters:
    ///
    /// none
    ///
    /// # Return values:
    ///
    /// - `entityCnt: u8`
    /// - `unitId: [u8; 4]` (device specific identifier)
    /// - `transport: [u8; 2]` (bitfield)
    /// - `modelId: [u8; 6]`
    ///
    /// [doc]: https://lekensteyn.nl/files/logitech/x0003_deviceinfo.html
    pub fn get_device_info<F>(
        &mut self,
    ) -> impl Future<Item = (u8, [u8; 4], [u8; 2], [u8; 6]), Error = oneshot::Canceled> {
        let request = [0x11, 0xff, 0x02, 0x01];
        self.raw_request(&request).map(|response| {
            let entity_cnt = response[0];
            let mut unit_id = [0; 4];
            unit_id.copy_from_slice(&response[1..5]);
            let mut transport = [0; 2];
            transport.copy_from_slice(&response[5..7]);
            let mut model_id = [0; 6];
            model_id.copy_from_slice(&response[7..13]);
            (entity_cnt, unit_id, transport, model_id)
        })
    }

    /// set button reporting mode
    ///
    /// # Parameters:
    ///
    /// - `report_buttons: u8` (boolean)
    ///
    /// # Return values:
    ///
    /// - `report_buttons: u8` (confirmation i guess?)
    pub fn set_report_buttons(
        &mut self,
        report_buttons: u8,
    ) -> impl Future<Item = (), Error = oneshot::Canceled> {
        let request = [0x11, 0xff, 0x05, 0x21, report_buttons];
        self.raw_request(&request)
            .map(move |response| assert_eq!(report_buttons, response[0]))
    }

    /// set light configuration
    ///
    /// # Parameters:
    ///
    /// - `light_config: LightConfig` (see light_config.rs)
    ///
    /// # Return values:
    ///
    /// - `old_config: LightConfig` (previous configuration)
    pub fn set_light_config(
        &mut self,
        light_config: LightConfig,
    ) -> impl Future<Item = (), Error = oneshot::Canceled> {
        let mut request = vec![0x11, 0xff, 0x04, 0x31];
        request.extend(light_config.as_bytes().iter());
        self.raw_request(&request).map(|_| ())
    }

    /// set sidetone volume
    ///
    /// # Parameters:
    ///
    /// - `volume: u8` (0x00 minimum, 0x64 maximum)
    ///
    /// # Return values:
    ///
    /// - `volume: u8` (same as params)
    pub fn set_sidetone_volume(
        &mut self,
        volume: u8,
    ) -> impl Future<Item = (), Error = oneshot::Canceled> {
        let request = [0x11, 0xff, 0x07, 0x11, volume];
        self.raw_request(&request)
            .map(move |response| assert_eq!(volume, response[0]))
    }
}

/// Enumerate and initialize devices
pub fn find_devices() -> Vec<Device> {
    let context = udev::Context::new().unwrap();

    let mut enumerator = udev::Enumerator::new(&context).unwrap();
    enumerator.match_subsystem("usb").unwrap();
    enumerator.match_attribute("idVendor", "046d").unwrap();
    enumerator.match_attribute("idProduct", "0a5b").unwrap();
    enumerator
        .scan_devices()
        .unwrap()
        .flat_map(|parent| {
            let mut enumerator = udev::Enumerator::new(&context).unwrap();
            enumerator.match_subsystem("hidraw").unwrap();
            enumerator.match_parent(&parent).unwrap();
            enumerator.scan_devices().unwrap().filter_map(|device| {
                if let Some(devnode) = device.devnode() {
                    Some(Device::new(devnode))
                } else {
                    None
                }
            })
        })
        .collect()
}
