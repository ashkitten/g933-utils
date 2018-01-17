extern crate clap;
extern crate futures;
extern crate libg933;

use clap::{App, SubCommand};
use futures::Future;

fn main() {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    let matches = App::new("g933control")
        .author("Ash Lea <ashlea@protonmail.com>")
        .about("Configure and control the Logitech G933 Gaming Headset")
        .subcommand(SubCommand::with_name("list")
            .about("List attached devices")
        )
        .subcommand(SubCommand::with_name("get")
            .about("Get a property of a device")
            .args_from_usage("
                -d, --device <device> 'Device to get property from'
                <property>            'Property to get'
            ")
        )
        .subcommand(SubCommand::with_name("set")
            .about("Set a property of a device")
            .args_from_usage("
                -d, --device <device> 'Device to set property on'
                <property>            'Property to set'
                <value>               'Value of property'
            ")
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {}

    if let Some(matches) = matches.subcommand_matches("get") {}

    if let Some(matches) = matches.subcommand_matches("set") {}

    println!(
        "{:?}",
        libg933::find_devices()[0].get_protocol_version().wait()
    );
}
