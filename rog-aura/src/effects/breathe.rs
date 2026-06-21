use serde::{Deserialize, Serialize};

use super::EffectState;
use crate::keyboard::{KeyLayout, LedCode};
use crate::{Colour, Speed};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Breathe {
    led: LedCode,
    /// The starting colour
    start_colour1: Colour,
    /// The secondary starting colour
    start_colour2: Colour,
    /// The speed at which to cycle between the colours
    speed: Speed,
    /// Temporary data to help keep state
    #[serde(skip)]
    colour: Colour,
    #[serde(skip)]
    count_flipped: bool,
    #[serde(skip)]
    use_colour1: bool,
}

impl Breathe {
    pub fn new(address: LedCode, colour1: Colour, colour2: Colour, speed: Speed) -> Self {
        Self {
            led: address,
            start_colour1: colour1,
            start_colour2: colour2,
            speed,
            colour: colour1,
            count_flipped: false,
            use_colour1: true,
        }
    }
}

impl EffectState for Breathe {
    fn get_colour(&self) -> Colour {
        self.colour
    }

    fn get_led(&self) -> LedCode {
        self.led
    }

    fn set_led(&mut self, address: LedCode) {
        self.led = address;
    }

    fn next_colour_state(&mut self, _layout: &KeyLayout) {
        let Self {
            start_colour1: colour1,
            start_colour2: colour2,
            speed,
            colour: colour_actual,
            count_flipped: flipped,
            use_colour1,
            ..
        } = self;

        let speed = 4 - <u8>::from(*speed);

        if *colour_actual == (Colour { r: 0, g: 0, b: 0 }) {
            *use_colour1 = !*use_colour1;
        }

        let colour = if !*use_colour1 { colour2 } else { colour1 };

        let r1_scale = colour.r / speed / 2;
        let g1_scale = colour.g / speed / 2;
        let b1_scale = colour.b / speed / 2;

        if *colour_actual == (Colour { r: 0, g: 0, b: 0 }) {
            *flipped = true;
        } else if colour_actual.r >= colour.r
            && colour_actual.g >= colour.g
            && colour_actual.b >= colour.b
        {
            *flipped = false;
        }

        if !*flipped {
            colour_actual.r = colour_actual.r.saturating_sub(r1_scale);
            colour_actual.g = colour_actual.g.saturating_sub(g1_scale);
            colour_actual.b = colour_actual.b.saturating_sub(b1_scale);
        } else {
            colour_actual.r = colour_actual.r.saturating_add(r1_scale);
            colour_actual.g = colour_actual.g.saturating_add(g1_scale);
            colour_actual.b = colour_actual.b.saturating_add(b1_scale);
        }
    }
}
