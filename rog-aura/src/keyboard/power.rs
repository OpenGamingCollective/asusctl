//! Power state for Laptop MCU RGB/LED. This is generally for newer
//! 0x18c6, 0x19B6, 0x1a30, keyboard models (2021+)
use std::fmt::Debug;
use std::ops::{BitAnd, BitOr};

use log::warn;
use serde::{Deserialize, Serialize};
#[cfg(feature = "dbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::aura_detection::LedSupportData;
use crate::{AuraDeviceType, PowerZones};

/// Meaning of this struct depends on the laptop generation.
/// - 2021+, the struct is a single zone with 4 states
/// - pre-2021, the struct is 1 or 2 zones and 3 states
/// - Tuf, the struct is 1 zone and 3 states
#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuraPowerState {
    pub zone: PowerZones,
    pub boot: bool,
    pub awake: bool,
    pub sleep: bool,
    /// Ignored for pre-2021 and Tuf
    pub shutdown: bool,
}

impl Default for AuraPowerState {
    /// Defaults all to off
    fn default() -> Self {
        Self {
            zone: PowerZones::Keyboard,
            boot: true,
            awake: true,
            sleep: true,
            shutdown: true,
        }
    }
}

impl AuraPowerState {
    fn default_for(zone: PowerZones) -> Self {
        Self {
            zone,
            boot: true,
            awake: true,
            sleep: true,
            shutdown: true,
        }
    }

    fn tuf_to_bytes(&self) -> Vec<u8> {
        // &cmd, &boot, &awake, &sleep, &keyboard
        vec![
            1, self.boot as u8, self.awake as u8, self.sleep as u8, 1,
        ]
    }

    /// # Bits for older 0x1866 keyboard model
    ///
    /// Keybord and Lightbar require Awake, Boot and Sleep apply to both
    /// Keybord and Lightbar regardless of if either are enabled (or Awake is
    /// enabled)
    ///
    /// |   Byte 1   |   Byte 2   |   Byte 3   | function |   hex    |
    /// |------------|------------|------------|----------|----------|
    /// | 0000, 0000 | 0000, 0000 | 0000, 0010 | Awake    | 00,00,02 |
    /// | 0000, 1000 | 0000, 0000 | 0000, 0000 | Keyboard | 08,00,00 |
    /// | 0000, 0100 | 0000, 0101 | 0000, 0000 | Lightbar | 04,05,00 |
    /// | 1100, 0011 | 0001, 0010 | 0000, 1001 | Boot/Sht | c3,12,09 |
    /// | 0011, 0000 | 0000, 1000 | 0000, 0100 | Sleep    | 30,08,04 |
    /// | 1111, 1111 | 0001, 1111 | 0000, 1111 | all on   |          |
    fn old_to_bytes(&self) -> Vec<u8> {
        let mut a: u32 = 0;
        if self.awake {
            a |= OldAuraPower::Awake as u32;
        }
        if self.boot {
            a |= OldAuraPower::Boot as u32;
        }
        if self.sleep {
            a |= OldAuraPower::Sleep as u32;
        }
        // Zone bits (Keyboard, Lightbar) should only be set when the zone
        // is actually active (awake). Otherwise OR'ing multiple per-zone
        // states together would always include zone bits from "off" states,
        // making it impossible to turn off a zone independently.
        if self.awake {
            if matches!(
                self.zone,
                PowerZones::Keyboard | PowerZones::KeyboardAndLightbar
            ) {
                a |= OldAuraPower::Keyboard as u32;
            }
            if matches!(
                self.zone,
                PowerZones::Lightbar | PowerZones::KeyboardAndLightbar
            ) {
                a |= OldAuraPower::Lightbar as u32;
            }
        }
        vec![
            ((a & 0xff0000) >> 16) as u8,
            ((a & 0xff00) >> 8) as u8,
            (a & 0xff) as u8,
            0x00,
        ]
    }

    pub fn new_to_byte(&self) -> u32 {
        match self.zone {
            PowerZones::Logo => {
                self.boot as u32
                    | ((self.awake as u32) << 2)
                    | ((self.sleep as u32) << 4)
                    | ((self.shutdown as u32) << 6)
            }
            PowerZones::Ally => {
                (self.boot as u32)
                    | ((self.awake as u32) << 1)
                    | ((self.sleep as u32) << 2)
                    | ((self.shutdown as u32) << 3)
            }
            PowerZones::Keyboard => {
                ((self.boot as u32) << 1)
                    | ((self.awake as u32) << 3)
                    | ((self.sleep as u32) << 5)
                    | ((self.shutdown as u32) << 7)
            }
            PowerZones::Lightbar => {
                ((self.boot as u32) << (7 + 2))
                    | ((self.awake as u32) << (7 + 3))
                    | ((self.sleep as u32) << (7 + 4))
                    | ((self.shutdown as u32) << (7 + 5))
            }
            PowerZones::Lid => {
                ((self.boot as u32) << (15 + 1))
                    | ((self.awake as u32) << (15 + 2))
                    | ((self.sleep as u32) << (15 + 3))
                    | ((self.shutdown as u32) << (15 + 4))
            }
            PowerZones::RearGlow => {
                ((self.boot as u32) << (23 + 1))
                    | ((self.awake as u32) << (23 + 2))
                    | ((self.sleep as u32) << (23 + 3))
                    | ((self.shutdown as u32) << (23 + 4))
            }
            PowerZones::None | PowerZones::KeyboardAndLightbar => 0,
        }
    }
}

#[cfg_attr(feature = "dbus", derive(Type, Value, OwnedValue))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaptopAuraPower {
    pub states: Vec<AuraPowerState>,
}

impl LaptopAuraPower {
    /// # Bits for newer 0x18c6, 0x19B6, 0x1a30, keyboard models
    ///
    /// | Byte 1 | Byte 2  | Byte 3  | Byte 4  | Label    |
    /// |--------|---------|---------|---------|----------|
    /// |00000001| 00000000| 00000000| 00000000|boot_logo_|
    /// |00000010| 00000000| 00000000| 00000000|boot_keyb_|
    /// |00000100| 00000000| 00000000| 00000000|awake_logo|
    /// |00001000| 00000000| 00000000| 00000000|awake_keyb|
    /// |00010000| 00000000| 00000000| 00000000|sleep_logo|
    /// |00100000| 00000000| 00000000| 00000000|sleep_keyb|
    /// |01000000| 00000000| 00000000| 00000000|shut_logo_|
    /// |10000000| 00000000| 00000000| 00000000|shut_keyb_|
    /// |00000000| 00000010| 00000000| 00000000|boot_bar__|
    /// |00000000| 00000100| 00000000| 00000000|awake_bar_|
    /// |00000000| 00001000| 00000000| 00000000|sleep_bar_|
    /// |00000000| 00010000| 00000000| 00000000|shut_bar__|
    /// |00000000| 00000000| 00000001| 00000000|boot_lid__|
    /// |00000000| 00000000| 00000010| 00000000|awkae_lid_|
    /// |00000000| 00000000| 00000100| 00000000|sleep_lid_|
    /// |00000000| 00000000| 00001000| 00000000|shut_lid__|
    /// |00000000| 00000000| 00000000| 00000001|boot_rear_|
    /// |00000000| 00000000| 00000000| 00000010|awake_rear|
    /// |00000000| 00000000| 00000000| 00000100|sleep_rear|
    /// |00000000| 00000000| 00000000| 00001000|shut_rear_|
    fn new_to_bytes(&self) -> Vec<u8> {
        let mut a: u32 = 0;
        for state in self.states.iter() {
            a |= state.new_to_byte();
        }
        vec![
            (a & 0xff) as u8,
            ((a & 0xff00) >> 8) as u8,
            ((a & 0xff0000) >> 16) as u8,
            ((a & 0xff000000) >> 24) as u8,
        ]
    }

    // TODO: use support data to setup correct zones
    pub fn new(aura_type: AuraDeviceType, support_data: &LedSupportData) -> Self {
        match aura_type {
            AuraDeviceType::Unknown | AuraDeviceType::Ally | AuraDeviceType::LaptopKeyboard2021 => {
                let mut states = Vec::new();
                for zone in support_data.power_zones.iter() {
                    states.push(AuraPowerState::default_for(*zone))
                }
                Self { states }
            }
            AuraDeviceType::LaptopKeyboardPre2021 => {
                // Create individual power states for each power zone.
                // Previously this collapsed Keyboard+Lightbar into a single
                // KeyboardAndLightbar zone, which prevented independent control.
                let mut states = Vec::new();
                for zone in support_data.power_zones.iter() {
                    states.push(AuraPowerState::default_for(*zone))
                }
                Self { states }
            }
            AuraDeviceType::LaptopKeyboardTuf => Self {
                states: vec![AuraPowerState::default_for(PowerZones::Keyboard)],
            },
            AuraDeviceType::ScsiExtDisk => todo!(),
            AuraDeviceType::AnimeOrSlash => todo!(),
        }
    }

    pub fn to_bytes(&self, aura_type: AuraDeviceType) -> Vec<u8> {
        if let Some(stuff) = self.states.first() {
            if stuff.zone == PowerZones::Ally {
                return vec![
                    0x5d,
                    0xd1,
                    0x09,
                    0x01,
                    stuff.new_to_byte() as u8,
                ];
            }
        }
        match aura_type {
            AuraDeviceType::LaptopKeyboard2021 | AuraDeviceType::Ally => self.new_to_bytes(),
            AuraDeviceType::LaptopKeyboardPre2021 => {
                if self.states.len() == 1 {
                    self.states
                        .first()
                        .cloned()
                        .unwrap_or_default()
                        .old_to_bytes()
                } else {
                    let mut bytes: Vec<Vec<u8>> =
                        self.states.iter().map(|s| s.old_to_bytes()).collect();
                    let mut b = bytes.pop().unwrap();
                    for i in bytes {
                        for (i, n) in i.iter().enumerate() {
                            b[i] |= n;
                        }
                    }
                    b
                }
            }
            AuraDeviceType::LaptopKeyboardTuf => self
                .states
                .first()
                .cloned()
                .unwrap_or_default()
                .tuf_to_bytes(),
            AuraDeviceType::Unknown => {
                warn!("Trying to create bytes for an unknown device");
                self.new_to_bytes()
            }
            AuraDeviceType::ScsiExtDisk => todo!("scsi disk not implemented yet"),
            AuraDeviceType::AnimeOrSlash => todo!("anime/slash not implemented yet"),
        }
    }
}

/// |   Byte 1   |   Byte 2   |   Byte 3   | function |   hex    |
/// |------------|------------|------------|----------|----------|
/// | 0000, 0000 | 0000, 0000 | 0000, 0010 | Awake    | 00,00,02 |
/// | 0000, 1000 | 0000, 0000 | 0000, 0000 | Keyboard | 08,00,00 |
/// | 0000, 0100 | 0000, 0101 | 0000, 0000 | Lightbar | 04,05,00 |
/// | 1100, 0011 | 0001, 0010 | 0000, 1001 | Boot/Sht | c3,12,09 |
/// | 0011, 0000 | 0000, 1000 | 0000, 0100 | Sleep    | 30,08,04 |
/// | 1111, 1111 | 0001, 1111 | 0000, 1111 | all on   |          |
#[repr(u32)]
enum OldAuraPower {
    Awake = 0x000002,
    Boot = 0xc31209,
    Sleep = 0x300804,
    Keyboard = 0x080000,
    Lightbar = 0x040500,
}

impl BitOr<OldAuraPower> for OldAuraPower {
    type Output = u32;

    fn bitor(self, rhs: OldAuraPower) -> Self::Output {
        self as u32 | rhs as u32
    }
}

impl BitAnd<OldAuraPower> for OldAuraPower {
    type Output = u32;

    fn bitand(self, rhs: OldAuraPower) -> Self::Output {
        self as u32 & rhs as u32
    }
}

impl From<OldAuraPower> for u32 {
    fn from(a: OldAuraPower) -> Self {
        a as u32
    }
}

#[cfg(test)]
mod test {
    use crate::keyboard::{AuraPowerState, LaptopAuraPower};
    use crate::{AuraDeviceType, PowerZones};

    fn to_binary_string_post2021(power: &LaptopAuraPower) -> String {
        let bytes = power.to_bytes(AuraDeviceType::LaptopKeyboard2021);
        format!(
            "{:08b}, {:08b}, {:08b}, {:08b}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        )
    }

    #[test]
    fn check_0x1866_control_bytes() {
        let power = LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        };
        let bytes = power.to_bytes(AuraDeviceType::LaptopKeyboardPre2021);
        println!("{:08b}, {:08b}, {:08b}", bytes[0], bytes[1], bytes[2]);
        assert_eq!(bytes, [0x08, 0x00, 0x02, 0x00]);

        let power = LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        };
        let bytes = power.to_bytes(AuraDeviceType::LaptopKeyboardPre2021);
        println!("{:08b}, {:08b}, {:08b}", bytes[0], bytes[1], bytes[2]);
        assert_eq!(bytes, [0x04, 0x05, 0x02, 0x00]);

        // let bytes = [
        //     OldAuraPower::Keyboard,
        //     OldAuraPower::Lightbar,
        //     OldAuraPower::Awake,
        //     OldAuraPower::Sleep,
        //     OldAuraPower::Boot,
        // ];
        let power = LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: true,
                    awake: true,
                    sleep: true,
                    shutdown: false,
                },
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: true,
                    awake: true,
                    sleep: true,
                    shutdown: false,
                },
            ],
        };
        let bytes = power.to_bytes(AuraDeviceType::LaptopKeyboardPre2021);
        println!("{:08b}, {:08b}, {:08b}", bytes[0], bytes[1], bytes[2]);
        assert_eq!(bytes, [0xff, 0x1f, 0x000f, 0x00]);
    }

    #[test]
    fn check_0x19b6_control_bytes_binary_rep() {
        let boot_logo_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Logo,
                    boot: true,
                    awake: false,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let boot_keyb_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: true,
                    awake: false,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let sleep_logo = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Logo,
                    boot: false,
                    awake: false,
                    sleep: true,
                    shutdown: false,
                },
            ],
        });
        let sleep_keyb = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: false,
                    awake: false,
                    sleep: true,
                    shutdown: false,
                },
            ],
        });
        let awake_logo = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Logo,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let awake_keyb = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let shut_logo_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Logo,
                    boot: false,
                    awake: false,
                    sleep: false,
                    shutdown: true,
                },
            ],
        });
        let shut_keyb_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    boot: false,
                    awake: false,
                    sleep: false,
                    shutdown: true,
                },
            ],
        });
        let boot_bar__ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: true,
                    awake: false,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let awake_bar_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let sleep_bar_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: false,
                    awake: false,
                    sleep: true,
                    shutdown: false,
                },
            ],
        });
        let shut_bar__ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    boot: false,
                    awake: false,
                    sleep: false,
                    shutdown: true,
                },
            ],
        });
        let boot_lid__ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lid,
                    boot: true,
                    awake: false,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let awake_lid_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lid,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let sleep_lid_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lid,
                    boot: false,
                    awake: false,
                    sleep: true,
                    shutdown: false,
                },
            ],
        });
        let shut_lid__ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Lid,
                    boot: false,
                    awake: false,
                    sleep: false,
                    shutdown: true,
                },
            ],
        });
        let boot_rear_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::RearGlow,
                    boot: true,
                    awake: false,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let awake_rear = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::RearGlow,
                    boot: false,
                    awake: true,
                    sleep: false,
                    shutdown: false,
                },
            ],
        });
        let sleep_rear = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::RearGlow,
                    boot: false,
                    awake: false,
                    sleep: true,
                    shutdown: false,
                },
            ],
        });
        let shut_rear_ = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::RearGlow,
                    boot: false,
                    awake: false,
                    sleep: false,
                    shutdown: true,
                },
            ],
        });

        assert_eq!(boot_logo_, "00000001, 00000000, 00000000, 00000000");
        assert_eq!(boot_keyb_, "00000010, 00000000, 00000000, 00000000");
        assert_eq!(awake_logo, "00000100, 00000000, 00000000, 00000000");
        assert_eq!(awake_keyb, "00001000, 00000000, 00000000, 00000000");
        assert_eq!(sleep_logo, "00010000, 00000000, 00000000, 00000000");
        assert_eq!(sleep_keyb, "00100000, 00000000, 00000000, 00000000");
        assert_eq!(shut_logo_, "01000000, 00000000, 00000000, 00000000");
        assert_eq!(shut_keyb_, "10000000, 00000000, 00000000, 00000000");
        //
        assert_eq!(boot_bar__, "00000000, 00000010, 00000000, 00000000");
        assert_eq!(awake_bar_, "00000000, 00000100, 00000000, 00000000");
        assert_eq!(sleep_bar_, "00000000, 00001000, 00000000, 00000000");
        assert_eq!(shut_bar__, "00000000, 00010000, 00000000, 00000000");
        //
        assert_eq!(boot_lid__, "00000000, 00000000, 00000001, 00000000");
        assert_eq!(awake_lid_, "00000000, 00000000, 00000010, 00000000");
        assert_eq!(sleep_lid_, "00000000, 00000000, 00000100, 00000000");
        assert_eq!(shut_lid__, "00000000, 00000000, 00001000, 00000000");
        //
        assert_eq!(boot_rear_, "00000000, 00000000, 00000000, 00000001");
        assert_eq!(awake_rear, "00000000, 00000000, 00000000, 00000010");
        assert_eq!(sleep_rear, "00000000, 00000000, 00000000, 00000100");
        assert_eq!(shut_rear_, "00000000, 00000000, 00000000, 00001000");

        // All on
        let byte1 = to_binary_string_post2021(&LaptopAuraPower {
            states: vec![
                AuraPowerState {
                    zone: PowerZones::Keyboard,
                    ..Default::default()
                },
                AuraPowerState {
                    zone: PowerZones::Lid,
                    ..Default::default()
                },
                AuraPowerState {
                    zone: PowerZones::Logo,
                    ..Default::default()
                },
                AuraPowerState {
                    zone: PowerZones::Lightbar,
                    ..Default::default()
                },
                AuraPowerState {
                    zone: PowerZones::RearGlow,
                    ..Default::default()
                },
            ],
        });
        assert_eq!(byte1, "11111111, 00011110, 00001111, 00001111");
    }
}
