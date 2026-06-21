use super::{EffectState, InputForEffect};
use crate::keyboard::{KeyLayout, LedCode};
use crate::Colour;

pub struct InputBased {
    led: LedCode,
    colour: Colour,
    /// - audio
    /// - cpu freq
    /// - temperature
    /// - fan speed
    /// - time
    input: Box<dyn InputForEffect>,
}

impl InputBased {
    pub fn new(led: LedCode, input: Box<dyn InputForEffect>) -> Self {
        Self {
            led,
            colour: Colour::default(),
            input,
        }
    }
}

impl EffectState for InputBased {
    fn next_colour_state(&mut self, _layout: &KeyLayout) {
        self.input.next_colour_state();
        self.colour = self.input.get_colour();
    }

    fn get_colour(&self) -> Colour {
        self.colour
    }

    fn get_led(&self) -> LedCode {
        self.led
    }

    fn set_led(&mut self, address: LedCode) {
        self.led = address;
    }
}
