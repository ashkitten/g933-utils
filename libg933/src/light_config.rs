use AsBytes;

#[derive(Copy, Clone)]
pub struct LightConfigOffSettings {}

#[derive(Copy, Clone)]
pub struct LightConfigBreathingSettings {
    pub rate: u16, // 0x4e20: slowest in software, 0x03e8: fastest in software
    pub padding0: u8,
    pub brightness: u8, // 0x01: dimmest in software, 0x64: brightest in software
    pub padding1: u8,
}

#[derive(Copy, Clone)]
pub struct LightConfigColorCycleSettings {
    pub padding0: [u8; 2],
    pub rate: u16,
    pub brightness: u8, // 0x01: dimmest in software, 0x64: brightest in software
}

pub union LightConfigSettings {
    pub off: LightConfigOffSettings,
    pub breathing: LightConfigBreathingSettings,
    pub color_cycle: LightConfigColorCycleSettings,
}

pub struct LightConfig {
    pub light_index: u8, // 0: logo, 1: sides
    pub effect: u8,      // 0: off, 1: static, 2: breathing, 3: color cycle
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub settings: LightConfigSettings,
    pub padding0: [u8; 2],
    pub profile_type: u8, // unknown exactly how this works, but 2 seems to be the "device profile" and 0 non-default
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
                    0 | 1 => vec![0u8; 5],
                    2 => {
                        let settings = self.settings.breathing;
                        vec![
                            (settings.rate >> 8) as u8,
                            (settings.rate & 0xff) as u8,
                            settings.padding0,
                            settings.brightness,
                            settings.padding1,
                        ]
                    }
                    3 => {
                        let settings = self.settings.color_cycle;
                        vec![
                            settings.padding0[0],
                            settings.padding0[1],
                            (settings.rate >> 8) as u8,
                            (settings.rate & 0xff) as u8,
                            settings.brightness,
                        ]
                    }
                    other => panic!("Effect index should not be greater than 3, was {}", other),
                }.iter(),
            );
        }

        params.extend(vec![self.padding0[0], self.padding0[1], self.profile_type].iter());

        params
    }
}
