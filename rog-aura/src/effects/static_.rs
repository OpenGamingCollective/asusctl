use serde::{Deserialize, Serialize};

use super::EffectState;
use crate::keyboard::{KeyLayout, LedCode};
use crate::Colour;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Static {
    led: LedCode,
    /// The starting colour
    colour: Colour,
}

impl Static {
    pub fn new(address: LedCode, colour: Colour) -> Self {
        Self {
            led: address,
            colour,
        }
    }
}

impl EffectState for Static {
    fn get_colour(&self) -> Colour {
        self.colour
    }

    fn get_led(&self) -> LedCode {
        self.led
    }

    fn set_led(&mut self, address: LedCode) {
        self.led = address;
    }

    fn next_colour_state(&mut self, _layout: &KeyLayout) {}
}
