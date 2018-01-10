extern crate hidapi;
extern crate libg933;

use libg933::*;

fn main() {
    let mut device = Device::new();
    device.set_report_buttons(0x01);

    device.process_data();
}
