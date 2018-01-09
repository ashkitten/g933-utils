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

extern crate hidapi;

use hidapi::HidDevice;

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
pub fn get_feature(device: &HidDevice, feature: u16) -> (u8, u8, u8) {
    let msb = (feature >> 8) as u8;
    let lsb = (feature & 0xff) as u8;

    let request = [0x11, 0xff, 0x00, 0x01, msb, lsb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    device.write(&request).unwrap();

    let mut response = [0u8; 20];
    loop {
        if device.read_timeout(&mut response, 1000).unwrap() == 0 {
            device.write(&request).unwrap();
        }
        if response[0..4] == [0x11, 0xff, 0x00, 0x01] {
            return (response[4], response[5], response[6]);
        }
    }
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
pub fn get_protocol_version(device: &HidDevice) -> (u8, u8) {
    let request = [0x11, 0xff, 0x00, 0x11, 0x00, 0x00, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    device.write(&request).unwrap();

    let mut response = [0u8; 20];
    loop {
        if device.read_timeout(&mut response, 1000).unwrap() == 0 {
            device.write(&request).unwrap();
        }
        if response[0..4] == [0x11, 0xff, 0x00, 0x11] {
            return (response[4], response[5]);
        }
    }
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
pub fn get_device_info(device: &HidDevice) -> (u8, [u8; 4], [u8; 2], [u8; 6]) {
    let request = [0x11, 0xff, 0x02, 0x01, 0x00, 0x00, 0xee, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    device.write(&request).unwrap();

    let mut response = [0u8; 20];
    loop {
        if device.read_timeout(&mut response, 1000).unwrap() == 0 {
            device.write(&request).unwrap();
        }
        if response[0..4] == [0x11, 0xff, 0x02, 0x01] {
            let entity_cnt = response[4];
            let mut unit_id = [0; 4];
            unit_id.copy_from_slice(&response[5..9]);
            let mut transport = [0; 2];
            transport.copy_from_slice(&response[9..11]);
            let mut model_id = [0; 6];
            model_id.copy_from_slice(&response[11..17]);
            return (entity_cnt, unit_id, transport, model_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use hidapi::HidApi;

    #[test]
    fn it_works() {
        let api = HidApi::new().unwrap();
        let device = api.open(0x046d, 0x0a5b).unwrap();

        println!("feature 0x0003: {:?}", ::get_feature(&device, 0x0003));
        println!("protocol version: {:?}", ::get_protocol_version(&device));
        println!("device info: {:?}", ::get_device_info(&device));
    }
}
