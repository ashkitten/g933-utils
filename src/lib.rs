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

/// Send a request to the device and return the result
pub fn send_request(device: &HidDevice, feature_index: u8, fnid_swid: u8, parameters: &[u8]) -> [u8; 16] {
    assert!(parameters.len() <= 16);

    let mut request = vec![0x11, 0xff, feature_index, fnid_swid];
    request.extend(parameters.iter());
    // Pad with zeros (might not be necessary)
    let len = request.len();
    request.extend(vec![0u8; 20 - len].iter());
    device.write(request.as_slice()).unwrap();

    let mut response = [0u8; 20];
    loop {
        // If it times out without reading anything, send our request again
        if device.read_timeout(&mut response, 2000).unwrap() == 0 {
            device.write(&request).unwrap();
        }

        if response[0..4] == [0x11, 0xff, feature_index, fnid_swid] {
            let mut ret = [0u8; 16];
            ret.copy_from_slice(&response[4..]);
            return ret;
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
pub fn get_feature(device: &HidDevice, feature: u16) -> (u8, u8, u8) {
    let response = send_request(device, 0x00, 0x01, &[(feature >> 8) as u8, (feature & 0xff) as u8]);
    (response[0], response[1], response[2])
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
    let response = send_request(device, 0x00, 0x11, &[0x00, 0x00, 0xee]);
    (response[0], response[1])
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
    let response = send_request(device, 0x02, 0x01, &[]);

    let entity_cnt = response[0];
    let mut unit_id = [0; 4];
    unit_id.copy_from_slice(&response[1..5]);
    let mut transport = [0; 2];
    transport.copy_from_slice(&response[5..7]);
    let mut model_id = [0; 6];
    model_id.copy_from_slice(&response[7..13]);
    return (entity_cnt, unit_id, transport, model_id);
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
fn set_report_buttons(device: &HidDevice, report_buttons: u8) {
    let response = send_request(device, 0x05, 0x21, &[report_buttons]);
    assert_eq!(response[0], report_buttons);
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
        ::set_report_buttons(&device, 0x01);
        println!("reporting button presses: true");
    }
}
