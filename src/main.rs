// Because otherwise clippy will warn us on use of format_err!, and I want to keep it consistent
#![cfg_attr(feature = "cargo-clippy", allow(useless_format))]

extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate indoc;
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
        .after_help(indoc!("
            Use --help with any subcommand for more information
        "))
        .subcommand(SubCommand::with_name("get")
            .about("Get a property of a device")
            .args_from_usage("
                -d, --device [device] 'Device to get property from'
                <property>            'Property to get'
            ")
            .after_help(indoc!("
                Valid options for `property` are:
                    battery
                    button_count
                    buttons
                    equalizer
                    equalizer_bands
                    poweroff_timeout
                    sidetone_volume
                    startup_effect
            "))
        )
        .subcommand(SubCommand::with_name("set")
            .about("Set a property of a device")
            .args_from_usage("
                -d, --device [device] 'Device to set property on'
                <property>            'Property to set'
                <value>...            'Value(s) of property'
            ")
            .after_help(indoc!("
                Valid options for `property` are:
                    buttons (bool)
                    equalizer (-12 - 11...)
                    poweroff_timeout (1 - 255 or 'never')
                    sidetone_volume (0 - 100)
                    startup_effect (bool)
            "))
        )
        .subcommand(SubCommand::with_name("watch")
            .about("Watch for events")
            .args_from_usage("
                -d, --device [device] 'Device to watch'
                <event>               'Event to watch for'
            ")
            .after_help(indoc!("
                Valid options for `event` are:
                    buttons
            "))
        )
        .subcommand(SubCommand::with_name("raw")
            .about("Send a raw request to a device")
            .args_from_usage("
                -d, --device [device] 'Device to send request to'
                -f, --format [format] 'Response format'
                <request>...          'Bytes of request separated by spaces'
            ")
            .after_help(indoc!("
                NOTE: The bytes of the request will always be parsed as base 16
            "))
        )
        .get_matches();

    if matches.subcommand_matches("list").is_some() {
        for (sysname, mut device) in libg933::find_devices()? {
            println!("Device {}: {}", sysname, device.get_device_name()?);
        }
    }

    if let Some(matches) = matches.subcommand_matches("get") {
        let property = matches.value_of("property").unwrap();
        let mut devices = libg933::find_devices()?;
        let mut device = match matches.value_of("device") {
            Some(sysname) => devices
                .get_mut(sysname)
                .ok_or_else(|| format_err!("No such device: {}", sysname))?,
            None => devices
                .values_mut()
                .next()
                .ok_or_else(|| format_err!("No devices found"))?,
        };

        match property {
            "battery" => {
                use libg933::battery::ChargingStatus::*;

                let battery_status = device.get_battery_status()?;
                let charging_status = match battery_status.charging_status {
                    Discharging => "discharging",
                    Charging(false) => "charging (ascending)",
                    Charging(true) => "charging (descending)",
                    Full => "full",
                };

                println!(
                    "Status: {:.01}% [{}]",
                    battery_status.charge, charging_status
                );
            }
            "button_count" => {
                println!("Number of buttons: {}", device.get_button_count()?);
            }
            "buttons" => {
                if device.get_buttons_enabled()? {
                    println!("Enabled");
                } else {
                    println!("Disabled");
                }
            }
            "equalizer" => {
                println!("Equalizer: {:?}", device.get_equalizer()?);
            }
            "equalizer_bands" => {
                println!("Bands (Hz): {:?}", device.get_equalizer_bands()?);
            }
            "poweroff_timeout" => {
                let timeout = match device.get_poweroff_timeout()? {
                    None => "never".to_string(),
                    Some(1) => "1 minute".to_string(),
                    Some(t) => format!("{} minutes", t),
                };

                println!("Timeout: {}", timeout);
            }
            "sidetone_volume" => {
                println!("Volume: {}%", device.get_sidetone_volume()?);
            }
            "startup_effect" => {
                if device.get_startup_effect_enabled()? {
                    println!("Enabled");
                } else {
                    println!("Disabled");
                }
            }
            p => println!("Invalid property: {}", p),
        }
    }

    if let Some(matches) = matches.subcommand_matches("set") {
        let property = matches.value_of("property").unwrap();
        let values: Vec<&str> = matches.values_of("value").unwrap().collect();
        let mut devices = libg933::find_devices()?;
        let mut device = match matches.value_of("device") {
            Some(sysname) => devices
                .get_mut(sysname)
                .ok_or_else(|| format_err!("No such device: {}", sysname))?,
            None => devices
                .values_mut()
                .next()
                .ok_or_else(|| format_err!("No devices found"))?,
        };

        match property {
            "buttons" => {
                let enable = values[0].parse::<bool>()?;
                device.enable_buttons(enable)?;
            }
            "equalizer" => {
                ensure!(values.len() <= 10, "Only 10 equalizer values allowed");

                // Start with the old config and overwrite parts
                let mut config = device.get_equalizer()?;
                for (i, value) in values.iter().enumerate() {
                    config[i] = i8::from_str_radix(value, 16).unwrap();
                }

                device.set_equalizer(false, config)?;
                device.set_equalizer(true, config)?;
            }
            "poweroff_timeout" => {
                let timeout = match values[0] {
                    "never" => None,
                    timeout => Some(timeout.parse::<u8>()?),
                };

                if let Some(0) = timeout {
                    bail!("Timeout must be greater than 0. If you want to never time out, specify 'never' instead");
                }

                device.set_poweroff_timeout(timeout)?;
            }
            "sidetone_volume" => {
                let volume = values[0].parse::<u8>()?;
                assert!(volume <= 100);
                device.set_sidetone_volume(volume)?;
            }
            "startup_effect" => {
                let enable = values[0].parse::<bool>()?;
                device.enable_startup_effect(enable)?;
            }
            p => println!("Invalid property: {}", p),
        }
    }

    if let Some(matches) = matches.subcommand_matches("watch") {
        let event = matches.value_of("event").unwrap();
        let mut devices = libg933::find_devices()?;
        let mut device = match matches.value_of("device") {
            Some(sysname) => devices
                .get_mut(sysname)
                .ok_or_else(|| format_err!("No such device: {}", sysname))?,
            None => devices
                .values_mut()
                .next()
                .ok_or_else(|| format_err!("No devices found"))?,
        };

        match event {
            "buttons" => {
                device.watch_buttons(|buttons| {
                    println!("g1: {}, g2: {}, g3: {}", buttons.g1, buttons.g2, buttons.g3);
                })?;
            }
            e => println!("Invalid event: {}", e),
        }
    }

    if let Some(matches) = matches.subcommand_matches("raw") {
        let format = matches.value_of("format").unwrap_or("bytes");
        let mut devices = libg933::find_devices()?;
        let mut device = match matches.value_of("device") {
            Some(sysname) => devices
                .get_mut(sysname)
                .ok_or_else(|| format_err!("No such device: {}", sysname))?,
            None => devices
                .values_mut()
                .next()
                .ok_or_else(|| format_err!("No devices found"))?,
        };

        let request = matches
            .values_of("request")
            .unwrap()
            .map(|byte| u8::from_str_radix(byte, 16).unwrap())
            .collect::<Vec<u8>>();

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
            "string" => println!(
                "{}",
                String::from_utf8_lossy(&device.raw_request(&request)?)
            ),
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
