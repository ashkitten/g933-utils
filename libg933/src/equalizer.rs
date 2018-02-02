//! Equalizer config stuff
// TODO: figure out what first byte is for

use {AsBytes, FromBytes};

/// Configuration for equalizer
#[derive(Debug)]
pub struct Config {
    /// Bands of equalizer
    /// Bands are: 32, 64, 125, 250, 500, 1k, 2k, 4k, 8k, and 16k Hz
    pub bands: [i8; 10],
}

impl AsBytes for Config {
    fn as_bytes(&self) -> Vec<u8> {
        let bands = unsafe { ::std::mem::transmute::<_, [u8; 10]>(self.bands) };
        v![2, @bands.iter()]
    }
}

impl FromBytes for Config {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut bands = [0u8; 10];
        bands.copy_from_slice(&bytes[1..11]);
        let bands = unsafe { ::std::mem::transmute::<_, [i8; 10]>(bands) };
        Self { bands }
    }
}
