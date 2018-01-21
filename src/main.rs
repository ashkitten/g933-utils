extern crate clap;
extern crate env_logger;
#[macro_use]
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
                -d, --device [device] 'Device to get property from'
                <property>            'Property to get'
            ")
        )
        .subcommand(SubCommand::with_name("set")
            .about("Set a property of a device")
            .args_from_usage("
                -d, --device [device] 'Device to set property on'
                <property>            'Property to set'
                <value>               'Value of property'
            ")
        )
        .subcommand(SubCommand::with_name("raw")
            .about("Send a raw request to a device")
            .args_from_usage("
                -d, --device [device] 'Device to send request to'
                -f, --format [format] 'Response format'
                <request>...          'Bytes of request separated by spaces'
            ")
            .after_help("\
                NOTE: The bytes of the request will always be parsed as base 16\n\
            ")
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        for (i, device) in libg933::find_devices()?.iter_mut().enumerate() {
            println!(
                "Device {}, protocol version: {:?}",
                i,
                device.get_protocol_version()?
            );
        }
    }

    if let Some(matches) = matches.subcommand_matches("get") {
        let devnum = matches.value_of("device").unwrap_or("0").parse::<usize>()?;
        let property = matches.value_of("property").unwrap();
        let mut device = libg933::find_devices()?.remove(devnum);

        match property {
            p => println!("Invalid property: {}", p),
        }
    }

    if let Some(matches) = matches.subcommand_matches("set") {
        let devnum = matches.value_of("device").unwrap_or("0").parse::<usize>()?;
        let property = matches.value_of("property").unwrap();
        let value = matches.value_of("value").unwrap();
        let mut device = libg933::find_devices()?.remove(devnum);

        match property {
            "report_buttons" => {
                let reports = value.parse::<bool>()?;
                device.set_report_buttons(reports)?;
            }
            "sidetone_volume" => {
                let volume = value.parse::<u8>()?;
                assert!(volume <= 100);
                device.set_sidetone_volume(volume)?;
            }
            p => println!("Invalid property: {}", p),
        }
    }

    if let Some(matches) = matches.subcommand_matches("raw") {
        let devnum = matches.value_of("device").unwrap_or("0").parse::<usize>()?;
        let format = matches.value_of("format").unwrap_or("bytes");
        let request = matches
            .values_of("request")
            .unwrap()
            .flat_map(|bytes| {
                bytes
                    .split_whitespace()
                    .map(|b| u8::from_str_radix(b, 16).unwrap())
            })
            .collect::<Vec<u8>>();
        let mut device = libg933::find_devices()?.remove(devnum);

        match format {
            "bytes" => println!(
                "{}",
                device
                    .raw_request(&request)?
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            "string" => println!("{}", String::from_utf8_lossy(&device.raw_request(&request)?)),
            format => bail!("Invalid format: {}", format),
        }
    }

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
