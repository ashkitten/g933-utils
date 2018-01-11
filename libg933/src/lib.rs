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

#![feature(drain_filter)]
#![warn(missing_docs)]

extern crate hidapi;
extern crate owning_ref;

pub mod util;
pub mod light_config;

use hidapi::{HidApi, HidDevice};
use std::ops::Deref;
use owning_ref::OwningHandle;

use util::DerefInner;
use light_config::*;

/// Convert a struct that implements this trait to bytes
pub trait AsBytes {
    /// Convert a struct that implements this trait to bytes
    fn as_bytes(&self) -> Vec<u8>;
}

/// A struct that contains the bytes of a request as well as a callback for when a response is
/// retrieved
pub struct Request {
    /// The request buffer
    buf: [u8; 20],
    /// A callback for when the response is retrieved
    callback: Box<Fn([u8; 16])>,
}

impl Request {
    /// Construct a new `Request`
    pub fn new<F>(feature_index: u8, fnid_swid: u8, parameters: &[u8], callback: F) -> Self
    where
        F: Fn([u8; 16]) + 'static,
    {
        let mut buf = [0u8; 20];
        buf[..4].copy_from_slice(&[0x11, 0xff, feature_index, fnid_swid]);
        buf[4..(4 + parameters.len())].copy_from_slice(parameters);

        Self {
            buf,
            callback: Box::new(callback),
        }
    }

    /// Retrieve the `feature_index` from the `Request`
    pub fn feature_index(&self) -> u8 {
        self.buf[2]
    }

    /// Retrieve the `fnid_swid` from the `Request`
    pub fn fnid_swid(&self) -> u8 {
        self.buf[3]
    }

    /// Deliver a response via callback
    pub fn respond(&self, response: [u8; 16]) {
        (self.callback)(response);
    }
}

impl Deref for Request {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

/// Contains a `HidDevice` and a vector of requests to be processed
pub struct Device<'a> {
    hid_device: OwningHandle<Box<HidApi>, DerefInner<HidDevice<'a>>>,
    requests: Vec<Request>,
}

impl<'a> Device<'a> {
    /// Construct a new `Device`
    pub fn new() -> Self {
        let hid_device = OwningHandle::new_with_fn(
            Box::new(HidApi::new().unwrap()),
            |api| unsafe { DerefInner((*api).open(0x046d, 0x0a5b).unwrap()) },
        );

        Self {
            hid_device,
            requests: Vec::new(),
        }
    }

    /// Process requests and returned data
    pub fn process_data(&mut self) {
        let mut data = [0u8; 20];
        while self.requests.len() != 0 {
            for request in &self.requests {
                self.hid_device.write(&**request).unwrap();
            }

            // If it times out without reading anything, loop again
            if self.hid_device.read_timeout(&mut data, 2000).unwrap() == 0 {
                continue;
            }

            for request in self.requests.drain_filter(|request| {
                data[0..4] == [0x11, 0xff, request.feature_index(), request.fnid_swid()]
            }) {
                let mut response = [0u8; 16];
                response.copy_from_slice(&data[4..]);
                request.respond(response);
            }
        }
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
    pub fn get_feature<F>(&mut self, feature: u16, callback: F)
    where
        F: Fn((u8, u8, u8)) + 'static,
    {
        self.requests.push(Request::new(
            0x00,
            0x01,
            &[(feature >> 8) as u8, (feature & 0xff) as u8],
            move |response| callback((response[0], response[1], response[2])),
        ));
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
    pub fn get_protocol_version<F>(&mut self, callback: F)
    where
        F: Fn((u8, u8)) + 'static,
    {
        self.requests.push(Request::new(
            0x00,
            0x11,
            &[0x00, 0x00, 0xee],
            move |response| callback((response[0], response[1])),
        ));
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
    pub fn get_device_info<F>(&mut self, callback: F)
    where
        F: Fn((u8, [u8; 4], [u8; 2], [u8; 6])) + 'static,
    {
        self.requests
            .push(Request::new(0x02, 0x01, &[], move |response| {
                let entity_cnt = response[0];
                let mut unit_id = [0; 4];
                unit_id.copy_from_slice(&response[1..5]);
                let mut transport = [0; 2];
                transport.copy_from_slice(&response[5..7]);
                let mut model_id = [0; 6];
                model_id.copy_from_slice(&response[7..13]);
                callback((entity_cnt, unit_id, transport, model_id));
            }));
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
    pub fn set_report_buttons(&mut self, report_buttons: u8) {
        self.requests.push(Request::new(
            0x05,
            0x21,
            &[report_buttons],
            move |response| {
                assert_eq!(response[0], report_buttons);
            },
        ));
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
    pub fn set_light_config(&mut self, light_config: LightConfig) {
        self.requests.push(Request::new(
            0x04,
            0x31,
            light_config.as_bytes().as_slice(),
            move |_response| {},
        ));
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
    pub fn set_sidetone_volume(&mut self, volume: u8) {
        self.requests.push(Request::new(
            0x07,
            0x11,
            &[volume],
            move |_response| {}
        ));
    }
}
