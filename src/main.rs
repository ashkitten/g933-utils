extern crate hidapi;
extern crate libg933;

use libg933::*;

fn main() {
    let mut device = Device::new();

    // TODO: arguments n shit

    device.process_data();
}
