extern crate hidapi;
extern crate libg933;

use libg933::*;
use libg933::light_config::*;

fn main() {
    let mut device = Device::new();
    device.set_report_buttons(0x01);
    device.set_light_config(LightConfig {
        light_index: 1,
        effect: 2,
        red: 255,
        green: 0,
        blue: 255,
        settings: LightConfigSettings {
            breathing: LightConfigBreathingSettings {
                rate: 300,
                padding0: 0x00,
                brightness: 100,
                padding1: 0x00,
            },
        },
        padding0: Default::default(),
        profile_type: 0,
    });

    device.process_data();
}
