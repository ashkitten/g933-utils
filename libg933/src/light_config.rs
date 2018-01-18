//! Configuration structs and stuff for headset lighting

use {AsBytes, FromBytes};

/// A union type for all light configuration settings structs
pub enum LightConfigSettings {
    /// Settings for the off effect
    Off,
    /// Settings for the breathing effect
    Breathing {
        rate: u16,
        brightness: u8,
    },
    /// Settings for the color cycle effect
    ColorCycle {
        rate: u16,
        brightness: u8,
    },
}

impl AsBytes for LightConfigSettings {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 5];
        match *self {
            LightConfigSettings::Off => bytes,
            LightConfigSettings::Breathing { rate, brightness } => {
                bytes[0] = (rate >> 8) as u8;
                bytes[1] = (rate & 0xff) as u8;
                bytes[3] = brightness;
                bytes
            },
            LightConfigSettings::ColorCycle { rate, brightness } => {
                bytes[2] = (rate >> 8) as u8;
                bytes[3] = (rate & 0xff) as u8;
                bytes[4] = brightness;
                bytes
            },
        }
    }
}

/// Headset light configuration
pub struct LightConfig {
    /// Index of the light to configure - 0: logo, 1: sides
    pub light_index: u8,
    /// Effect index - 0: off, 1: static, 2: breathing, 3: color cycle
    pub effect: u8,
    /// Red color value
    pub red: u8,
    /// Green color value
    pub green: u8,
    /// Blue color value
    pub blue: u8,
    /// Extra settings for the effect
    pub settings: LightConfigSettings,
    /// Profile type - unknown exactly how this works, but 2 seems to be the "device profile" and 0 non-default
    pub profile_type: u8,
}

impl AsBytes for LightConfig {
    fn as_bytes(&self) -> Vec<u8> {
        let mut params = vec![
            self.light_index,
            self.effect,
            self.red,
            self.green,
            self.blue,
        ];

        params.extend(self.settings.as_bytes().iter());

        params.extend([0, 0, self.profile_type].iter());

        params
    }
}

impl FromBytes for LightConfig {
    fn from_bytes(bytes: &Vec<u8>) -> Self {
        Self {
            light_index: bytes[0],
            effect: bytes[1],
            red: bytes[2],
            green: bytes[3],
            blue: bytes[4],
            settings: match bytes[1] {
                0 => LightConfigSettings::Off,
                1 => LightConfigSettings::Breathing {
                    rate: ((bytes[5] as u16) << 8) & (bytes[6] as u16),
                    brightness: bytes[8],
                },
                2 => LightConfigSettings::ColorCycle {
                    rate: ((bytes[7] as u16) << 8) & (bytes[8] as u16),
                    brightness: bytes[9],
                },
                _ => unreachable!(),
            },
            profile_type: bytes[11],
        }
    }
}
