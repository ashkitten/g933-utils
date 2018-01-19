//! Configuration structs and stuff for headset lighting

use {AsBytes, FromBytes};

/// Describes which light to configure
#[derive(Debug)]
pub enum Light {
    /// The logo light
    Logo,
    /// The main lights on the side
    Side,
}

/// Configuration for the light effect
#[derive(Debug)]
pub enum Effect {
    /// Settings for the off effect
    Off,
    /// Settings for the static color effect
    Static {
        /// Red value
        red: u8,
        /// Blue value
        green: u8,
        /// Green value
        blue: u8,
    },
    /// Settings for the breathing effect
    Breathing {
        /// Red value
        red: u8,
        /// Green value
        green: u8,
        /// Blue value
        blue: u8,
        /// The rate of the breathing effect
        rate: u16,
        /// Light brightness
        brightness: u8,
    },
    /// Settings for the color cycle effect
    ColorCycle {
        /// The rate of the cycle effect
        rate: u16,
        /// Light brightness
        brightness: u8,
    },
}

/// Headset light configuration
#[derive(Debug)]
pub struct Config {
    /// Which light to configure
    pub light: Light,
    /// Configuration for the effect
    pub effect: Effect,
    /// Profile type - unknown exactly how this works, but 2 seems to be the "device profile" and 0 non-default
    pub profile_type: u8,
}

impl AsBytes for Config {
    fn as_bytes(&self) -> Vec<u8> {
        let mut params = vec![0u8; 12];

        params[0] = match self.light {
            Light::Logo => 0x00,
            Light::Side => 0x01,
        };

        params[1] = match self.effect {
            Effect::Off => 0x00,
            Effect::Static { .. } => 0x01,
            Effect::Breathing { .. } => 0x02,
            Effect::ColorCycle { .. } => 0x03,
        };

        match self.effect {
            Effect::Off => (),
            Effect::Static { red, green, blue } => {
                params[2] = red;
                params[3] = green;
                params[4] = blue;
            },
            Effect::Breathing { red, green, blue, rate, brightness } => {
                params[2] = red;
                params[3] = green;
                params[4] = blue;
                params[5] = (rate >> 8) as u8;
                params[6] = (rate & 0xff) as u8;
                params[8] = brightness;
            }
            Effect::ColorCycle { rate, brightness } => {
                params[7] = (rate >> 8) as u8;
                params[8] = (rate & 0xff) as u8;
                params[9] = brightness;
            }
        };

        params[11] = self.profile_type;

        params
    }
}

impl FromBytes for Config {
    fn from_bytes(bytes: &Vec<u8>) -> Self {
        assert!(bytes[0] <= 1);
        assert!(bytes[1] <= 3);

        Self {
            light: match bytes[0] {
                0 => Light::Logo,
                1 => Light::Side,
                _ => unreachable!(),
            },
            effect: match bytes[1] {
                0 => Effect::Off,
                1 => Effect::Static {
                    red: bytes[2],
                    green: bytes[3],
                    blue: bytes[4],
                },
                2 => Effect::Breathing {
                    red: bytes[2],
                    green: bytes[3],
                    blue: bytes[4],
                    rate: ((bytes[5] as u16) << 8) & (bytes[6] as u16),
                    brightness: bytes[8],
                },
                3 => Effect::ColorCycle {
                    rate: ((bytes[7] as u16) << 8) & (bytes[8] as u16),
                    brightness: bytes[9],
                },
                _ => unreachable!(),
            },
            profile_type: bytes[11],
        }
    }
}
