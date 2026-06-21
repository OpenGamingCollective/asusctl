use serde::{Deserialize, Serialize};

mod doom;
pub use doom::*;

mod base;
pub use base::*;

mod breathe;
pub use breathe::*;

mod static_;
pub use static_::*;

use crate::keyboard::{AuraLaptopUsbPackets, KeyLayout, LedCode, LedUsbPackets};
use crate::Colour;

// static mut RNDINDEX: usize = 0;
static mut PRNDINDEX: usize = 0;

/// Pseudo random table ripped straight out of room4doom
pub const RNDTABLE: [i32; 256] = [
    0, 8, 109, 220, 222, 241, 149, 107, 75, 248, 254, 140, 16, 66, 74, 21, 211, 47, 80, 242, 154,
    27, 205, 128, 161, 89, 77, 36, 95, 110, 85, 48, 212, 140, 211, 249, 22, 79, 200, 50, 28, 188,
    52, 140, 202, 120, 68, 145, 62, 70, 184, 190, 91, 197, 152, 224, 149, 104, 25, 178, 252, 182,
    202, 182, 141, 197, 4, 81, 181, 242, 145, 42, 39, 227, 156, 198, 225, 193, 219, 93, 122, 175,
    249, 0, 175, 143, 70, 239, 46, 246, 163, 53, 163, 109, 168, 135, 2, 235, 25, 92, 20, 145, 138,
    77, 69, 166, 78, 176, 173, 212, 166, 113, 94, 161, 41, 50, 239, 49, 111, 164, 70, 60, 2, 37,
    171, 75, 136, 156, 11, 56, 42, 146, 138, 229, 73, 146, 77, 61, 98, 196, 135, 106, 63, 197, 195,
    86, 96, 203, 113, 101, 170, 247, 181, 113, 80, 250, 108, 7, 255, 237, 129, 226, 79, 107, 112,
    166, 103, 241, 24, 223, 239, 120, 198, 58, 60, 82, 128, 3, 184, 66, 143, 224, 145, 224, 81,
    206, 163, 45, 63, 90, 168, 114, 59, 33, 159, 95, 28, 139, 123, 98, 125, 196, 15, 70, 194, 253,
    54, 14, 109, 226, 71, 17, 161, 93, 186, 87, 244, 138, 20, 52, 123, 251, 26, 36, 17, 46, 52,
    231, 232, 76, 31, 221, 84, 37, 216, 165, 212, 106, 197, 242, 98, 43, 39, 175, 254, 145, 190,
    84, 118, 222, 187, 136, 120, 163, 236, 249,
];

pub fn p_random() -> i32 {
    unsafe {
        PRNDINDEX = (PRNDINDEX + 1) & 0xff;
        RNDTABLE[PRNDINDEX]
    }
}

pub trait InputForEffect {
    /// Calculate the next colour state
    fn next_colour_state(&mut self);

    /// Return the resulting colour. Implementers should store the colour to
    /// return it.
    fn get_colour(&self) -> Colour;
}

pub(crate) trait EffectState {
    /// Calculate the next colour state
    fn next_colour_state(&mut self, _layout: &KeyLayout);

    /// Return the resulting colour. Implementers should store the colour to
    /// return it.
    fn get_colour(&self) -> Colour;

    fn get_led(&self) -> LedCode;

    fn set_led(&mut self, address: LedCode);
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AdvancedEffects {
    effects: Vec<Effect>,
    zoned: bool,
}

impl AdvancedEffects {
    #[inline]
    pub fn new(zoned: bool) -> Self {
        Self {
            effects: Default::default(),
            zoned,
        }
    }

    #[inline]
    pub fn push(&mut self, action: Effect) {
        self.effects.push(action);
    }

    #[inline]
    pub fn insert(&mut self, index: usize, action: Effect) {
        self.effects.insert(index, action);
    }

    /// Remove an item at this position from the run buffer. If the `index`
    /// supplied is not in range then `None` is returned, otherwise the
    /// `ActionData` at that location is yeeted and returned.
    #[inline]
    pub fn remove_item(&mut self, index: usize) -> Option<Effect> {
        if index < self.effects.len() {
            return Some(self.effects.remove(index));
        }
        None
    }

    pub fn next_state(&mut self, layout: &KeyLayout) {
        for effect in &mut self.effects {
            effect.next_state(layout);
        }
    }

    pub fn create_packets(&self) -> AuraLaptopUsbPackets {
        let mut usb_packets = if self.zoned {
            // TODO: figure out if that single byte difference for multizone actually
            // matters
            LedUsbPackets::new_zoned(true)
        } else {
            LedUsbPackets::new_per_key()
        };

        for effect in &self.effects {
            let c = effect.colour();
            usb_packets.set(effect.led(), c.r, c.g, c.b);
        }
        usb_packets.into()
    }
}

/// A helper macro to quickly add new effects to the matching on `Effect`
macro_rules! effect_impl {
    ($($effect:ident),*) => {
        impl Effect {
            /// Get the type of LED set
            pub fn led(&self) -> $crate::keyboard::LedCode {
                match self {
                    $(Effect::$effect(c) => c.get_led(),)*
                }
            }

            /// Change the led type (can be used to change location of the effect)
            pub fn set_led(&mut self, address: $crate::keyboard::LedCode) {
                match self {
                    $(Effect::$effect(c) => c.set_led(address),)*
                }
            }

            /// Calculate the next state of the effect
            pub fn next_state(&mut self, layout: &KeyLayout) {
                match self {
                    $(Effect::$effect(c) => c.next_colour_state(layout),)*
                }
            }

            /// Get the calculated colour
            pub fn colour(&self) -> $crate::Colour {
                match self {
                    $(Effect::$effect(c) => c.get_colour(),)*
                }
            }
        }
    };
}

/// The main effect container, a sequencer will call various methods on this to
/// update states and get colours
///
/// Every effect is added here to quickly and easily match within `Effect`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Effect {
    Static(Static),
    Breathe(Breathe),
    DoomFlicker(DoomFlicker),
    DoomLightFlash(DoomLightFlash),
    // InputBased is not in the Effect enum because it contains Box<dyn InputForEffect>
    // which can't derive Debug/Clone/Serialize
}

impl Default for Effect {
    fn default() -> Self {
        Self::Static(Static::new(LedCode::default(), Colour::default()))
    }
}

effect_impl!(Static, Breathe, DoomFlicker, DoomLightFlash);

#[cfg(test)]
mod tests {
    use crate::effects::{AdvancedEffects, Breathe, DoomFlicker, Effect, Static};
    use crate::keyboard::{KeyLayout, LedCode};
    use crate::{Colour, Speed};

    #[test]
    fn single_key_next_state_then_create() {
        let layout = KeyLayout::default_layout();
        let mut seq = AdvancedEffects::new(false);
        seq.effects.push(Effect::Static(Static::new(
            LedCode::F,
            Colour {
                r: 255,
                g: 127,
                b: 0,
            },
        )));

        seq.next_state(&layout);
        let packets = seq.create_packets();

        assert_eq!(packets[0][0], 0x5d);
        assert_eq!(packets[5][33], 255);
        assert_eq!(packets[5][34], 127);
        assert_eq!(packets[5][35], 0);
    }

    #[test]
    fn cycle_breathe() {
        let layout = KeyLayout::default_layout();
        let mut seq = AdvancedEffects::new(false);
        seq.effects.push(Effect::Breathe(Breathe::new(
            LedCode::F,
            Colour {
                r: 255,
                g: 127,
                b: 0,
            },
            Colour {
                r: 127,
                g: 0,
                b: 255,
            },
            Speed::Med,
        )));

        let s =
            ron::ser::to_string_pretty(&seq, ron::ser::PrettyConfig::new().depth_limit(4)).unwrap();
        println!("{s}");

        seq.next_state(&layout);
        let packets = seq.create_packets();

        assert_eq!(packets[0][0], 0x5d);
        assert_eq!(packets[5][33], 213);
        assert_eq!(packets[5][34], 106);
        assert_eq!(packets[5][35], 0);

        // dbg!(&packets[5][33..=35]);

        seq.next_state(&layout);
        let packets = seq.create_packets();

        assert_eq!(packets[0][0], 0x5d);
        assert_eq!(packets[5][33], 171);
        assert_eq!(packets[5][34], 85);
        assert_eq!(packets[5][35], 0);
    }

    #[test]
    fn cycle_flicker() {
        let layout = KeyLayout::default_layout();
        let mut seq = AdvancedEffects::new(false);
        seq.effects.push(Effect::DoomFlicker(DoomFlicker::new(
            LedCode::F,
            Colour {
                r: 255,
                g: 127,
                b: 80,
            },
            100,
            10,
        )));

        seq.next_state(&layout);
        let packets = seq.create_packets();

        assert_eq!(packets[0][0], 0x5d);
        assert_eq!(packets[5][33], 255);
        assert_eq!(packets[5][34], 127);
        assert_eq!(packets[5][35], 80);

        // The random is deterministic
        seq.next_state(&layout);
        seq.next_state(&layout);
        seq.next_state(&layout);
        seq.next_state(&layout);
        seq.next_state(&layout);
        seq.next_state(&layout);
        seq.next_state(&layout);

        let packets = seq.create_packets();
        assert_eq!(packets[5][33], 215);
        assert_eq!(packets[5][34], 87);
        assert_eq!(packets[5][35], 40);
    }
}
