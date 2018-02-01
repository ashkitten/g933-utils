//! Structs for device information

use FromBytes;

/// Contains device information
#[derive(Debug)]
pub struct DeviceInfo {
    entity_count: u8,
    unit_id: [u8; 4],
    transport: [u8; 2],
    model_id: [u8; 6],
}

impl FromBytes for DeviceInfo {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            entity_count: bytes[0],
            unit_id: {
                let mut unit_id = [0; 4];
                unit_id.copy_from_slice(&bytes[1..5]);
                unit_id
            },
            transport: {
                let mut transport = [0; 2];
                transport.copy_from_slice(&bytes[5..7]);
                transport
            },
            model_id: {
                let mut model_id = [0; 6];
                model_id.copy_from_slice(&bytes[7..13]);
                model_id
            },
        }
    }
}
