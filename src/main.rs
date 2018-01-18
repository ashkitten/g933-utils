extern crate clap;
extern crate env_logger;
extern crate failure;
extern crate libg933;
#[macro_use]
extern crate log;

use clap::{App, SubCommand};
use failure::Error;

fn run() -> Result<(), Error> {
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

    if let Some(_) = matches.subcommand_matches("list") {
        for (i, device) in libg933::find_devices()?.iter_mut().enumerate() {
            println!("Device {}, protocol version: {:?}", i, device.get_protocol_version()?);
        }
    }

    if let Some(matches) = matches.subcommand_matches("get") {}

    if let Some(matches) = matches.subcommand_matches("set") {}

    Ok(())
}

fn main() {
    use std::io::Write;

    env_logger::init().expect("Failed to initialize logger");

    ::std::process::exit(match run() {
        Ok(()) => 0,
        Err(ref error) => {
            let mut causes = error.causes();

            error!(
                "{}",
                causes
                    .next()
                    .expect("`causes` should contain at least one error")
            );
            for cause in causes {
                error!("Caused by: {}", cause);
            }

            let backtrace = format!("{}", error.backtrace());
            if backtrace.is_empty() {
                writeln!(
                    ::std::io::stderr(),
                    "Set RUST_BACKTRACE=1 to see a backtrace"
                ).expect("Could not write to stderr");
            } else {
                writeln!(::std::io::stderr(), "{}", error.backtrace())
                    .expect("Could not write to stderr");
            }

            1
        }
    });
}
