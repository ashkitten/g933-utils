//! Battery-related code and stuff

use byteorder::{BigEndian, ByteOrder};
use std::collections::{BTreeMap, HashMap};

use FromBytesWithDevice;
use StaticDeviceMatch;

macro_rules! mapset {
    ($maps:expr, $pid:expr, $str:expr) => {
        {
            $maps.insert(
                ($pid, ChargingStatus::Discharging),
                make_map(include_str!(concat!("maps/", $str, "/discharging.csv"))));
            $maps.insert(
                ($pid, ChargingStatus::Charging(false)),
                make_map(include_str!(concat!("maps/", $str, "/charging_ascending.csv"))));
            $maps.insert(
                ($pid, ChargingStatus::Charging(true)),
                make_map(include_str!(concat!("maps/", $str, "/charging_descending.csv"))));
            $maps.insert(
                ($pid, ChargingStatus::Full), {
                let mut map = BTreeMap::new();
                map.insert(0, 100.0);
                map.insert(1, 100.0);
                map
            });
        }
    };
}

lazy_static! {
    static ref VOLTAGE_MAPS: HashMap<(u16, ChargingStatus), BTreeMap<isize, f32>> = {
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

        mapset!(maps, 0x0A5B, "0A5B");
        mapset!(maps, 0x0A66, "0A66");


        maps
    };
}

/// Charging status
#[derive(Debug, Eq, Hash, PartialEq, Copy, Clone)]
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

impl FromBytesWithDevice for BatteryStatus {
    fn from_bytes(dev: StaticDeviceMatch, bytes: &[u8]) -> Option<Self> {
        let charging_status = match bytes[2] {
            1 => ChargingStatus::Discharging,
            3 => ChargingStatus::Charging(false), // TODO: implement check for ascending/descending
            7 => ChargingStatus::Full,
            s => panic!("Encountered unknown charging status: {}", s),
        };

        debug!("Charging status: {:?}", charging_status);

        let voltage = BigEndian::read_u16(&bytes[0..2]) as isize;

        debug!("Voltage: {}", voltage);

        let key = &(dev.pid, charging_status);
        if !VOLTAGE_MAPS.contains_key(key) {
            return None;
        }
        let map = &VOLTAGE_MAPS[key];

        let closest_voltages = {
            let mut closest = (isize::max_value(), isize::max_value());
            for v in map.keys() {
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

        let closest_charges = (map[&closest_voltages.0], map[&closest_voltages.1]);

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

        Some(Self {
            charging_status,
            voltage: voltage as u16,
            charge,
        })
    }
}
