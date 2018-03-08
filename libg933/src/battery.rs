//! Battery-related code and stuff

use byteorder::{BigEndian, ByteOrder};
use std::collections::{BTreeMap, HashMap};

use FromBytes;

lazy_static! {
    static ref VOLTAGE_MAPS: HashMap<ChargingStatus, BTreeMap<isize, f32>> = {
        let mut maps = HashMap::new();

        fn make_map(input: &str) -> BTreeMap<isize, f32> {
            let mut map = BTreeMap::new();

            for mut split in input
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(|line| line.splitn(2, ','))
            {
                map.insert(
                    split.next().unwrap().parse::<isize>().unwrap(),
                    split.next().unwrap().parse::<f32>().unwrap(),
                );
            }

            map
        };

        maps.insert(
            ChargingStatus::Discharging,
            make_map(include_str!("discharging.csv")),
        );

        maps.insert(
            ChargingStatus::Charging(false),
            make_map(include_str!("charging_ascending.csv")),
        );

        maps.insert(
            ChargingStatus::Charging(true),
            make_map(include_str!("charging_descending.csv")),
        );

        maps.insert(ChargingStatus::Full, {
            let mut map = BTreeMap::new();
            map.insert(0, 100.0);
            map.insert(1, 100.0);
            map
        });

        maps
    };
}

/// Charging status
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ChargingStatus {
    /// Battery is discharging
    Discharging,
    /// Battery is charging - contains false when ascending, true when descending
    Charging(bool),
    /// Battery is full
    Full,
}

/// Battery status
#[derive(Debug)]
pub struct BatteryStatus {
    /// Charging status
    pub charging_status: ChargingStatus,
    /// Battery voltage
    pub voltage: u16,
    /// Charge percentage
    pub charge: f32,
}

impl FromBytes for BatteryStatus {
    fn from_bytes(bytes: &[u8]) -> Self {
        let charging_status = match bytes[2] {
            1 => ChargingStatus::Discharging,
            3 => ChargingStatus::Charging(false), // TODO: implement check for ascending/descending
            7 => ChargingStatus::Full,
            s => panic!("Encountered unknown charging status: {}", s),
        };

        debug!("Charging status: {:?}", charging_status);

        let voltage = BigEndian::read_u16(&bytes[0..2]) as isize;

        debug!("Voltage: {}", voltage);

        let closest_voltages = {
            let mut closest = (isize::max_value(), isize::max_value());
            for v in VOLTAGE_MAPS[&charging_status].keys() {
                // Insert in reverse to get them in increasing order
                if (*v - voltage).abs() < (closest.1 - voltage).abs() {
                    closest.0 = closest.1;
                    closest.1 = *v;
                }
            }
            closest
        };

        debug!(
            "Closest mapped voltages: {}, {}",
            closest_voltages.0, closest_voltages.1
        );

        let closest_charges = (
            VOLTAGE_MAPS[&charging_status][&closest_voltages.0],
            VOLTAGE_MAPS[&charging_status][&closest_voltages.1],
        );

        debug!(
            "Corresponding charges: {}, {}",
            closest_charges.0, closest_charges.1
        );

        let slope = ((voltage - closest_voltages.0) as f32)
            / ((closest_voltages.1 - closest_voltages.0) as f32);

        debug!("Voltage graph slope: {}", slope);

        let charge = (((closest_charges.1 - closest_charges.0) * slope) + closest_charges.0)
            .max(0.0)
            .min(100.0);

        debug!("Charge: {}", charge);

        Self {
            charging_status,
            voltage: voltage as u16,
            charge,
        }
    }
}
