use {AsBytes, FromBytes};

pub struct Buttons {
    g1: bool,
    g2: bool,
    g3: bool,
}

impl AsBytes for Buttons {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8];
        if self.g1 {
            bytes[0] += 1;
        }
        if self.g2 {
            bytes[0] += 2;
        }
        if self.g3 {
            bytes[0] += 4;
        }
        bytes
    }
}

impl FromBytes for Buttons {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            g1: bytes[0] & 1 != 0,
            g2: bytes[0] & 2 != 0,
            g3: bytes[0] & 4 != 0,
        }
    }
}
