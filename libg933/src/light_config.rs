//! Configuration structs and stuff for headset lighting

use AsBytes;

/// Settings struct for the off effect
#[derive(Copy, Clone, Default)]
pub struct LightConfigOffSettings {
    /// Padding
    pub padding0: [u8; 5],
}

impl AsBytes for LightConfigOffSettings {
    fn as_bytes(&self) -> Vec<u8> {
        self.padding0.iter().cloned().collect()
    }
}

/// Settings struct for the breathing effect
#[derive(Copy, Clone, Default)]
pub struct LightConfigBreathingSettings {
    /// Effect rate - 0x4e20: slowest in software, 0x03e8: fastest in software
    pub rate: u16,
    /// Padding
    pub padding0: u8,
    /// Light brightness - 0x01: dimmest in software, 0x64: brightest in software
    pub brightness: u8,
    /// Padding
    pub padding1: u8,
}

impl AsBytes for LightConfigBreathingSettings {
    fn as_bytes(&self) -> Vec<u8> {
        vec![
            (self.rate >> 8) as u8,
            (self.rate & 0xff) as u8,
            self.padding0,
            self.brightness,
            self.padding1,
        ]
    }
}

/// Settings struct for the color cycle effect
#[derive(Copy, Clone, Default)]
pub struct LightConfigColorCycleSettings {
    /// Padding
    pub padding0: [u8; 2],
    /// Effect rate - 0x4e20: slowest in software, 0x03e8: fastest in software
    pub rate: u16,
    /// Light brightness - 0x01: dimmest in software, 0x64: brightest in software
    pub brightness: u8,
}

impl AsBytes for LightConfigColorCycleSettings {
    fn as_bytes(&self) -> Vec<u8> {
        vec![
            self.padding0[0],
            self.padding0[1],
            (self.rate >> 8) as u8,
            (self.rate & 0xff) as u8,
            self.brightness,
        ]
    }
}

/// A union type for all light configuration settings structs
pub union LightConfigSettings {
    /// Settingss for the off effect
    pub off: LightConfigOffSettings,
    /// Settings for the breathing effect
    pub breathing: LightConfigBreathingSettings,
    /// Settings for the color cycle effect
    pub color_cycle: LightConfigColorCycleSettings,
}

impl Default for LightConfigSettings {
    fn default() -> Self {
        Self {
            off: Default::default(),
        }
    }
}

/// Headset light configuration
#[derive(Default)]
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
    /// Some padding
    pub padding0: [u8; 2], // TODO: figure out what's there
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

        unsafe {
            params.extend(
                match self.effect {
                    0 | 1 => self.settings.off.as_bytes(),
                    2 => self.settings.breathing.as_bytes(),
                    3 => self.settings.color_cycle.as_bytes(),
                    other => panic!("Effect index should not be greater than 3, was {}", other),
                }.iter(),
            );
        }

        params.extend(vec![self.padding0[0], self.padding0[1], self.profile_type].iter());

        params
    }
}
