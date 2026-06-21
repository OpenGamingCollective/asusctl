use serde::{Deserialize, Serialize};

use crate::effects::{p_random, EffectState};
use crate::keyboard::{KeyLayout, LedCode};
use crate::Colour;

/// Compute max/min light colours from a base colour and percentage values.
fn compute_light_range(base: Colour, max_percentage: u8, min_percentage: u8) -> (Colour, Colour) {
    let max_light = Colour {
        r: (base.r as f32 / 100.0 * max_percentage as f32) as u8,
        g: (base.g as f32 / 100.0 * max_percentage as f32) as u8,
        b: (base.b as f32 / 100.0 * max_percentage as f32) as u8,
    };

    // min light is a percentage of the set colour
    let min_light = Colour {
        r: (base.r as f32 / 100.0 * min_percentage as f32) as u8,
        g: (base.g as f32 / 100.0 * min_percentage as f32) as u8,
        b: (base.b as f32 / 100.0 * min_percentage as f32) as u8,
    };
    (max_light, min_light)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DoomFlicker {
    led: LedCode,
    start_colour: Colour,
    max_percentage: u8,
    min_percentage: u8,
    #[serde(skip)]
    count: u8,
    #[serde(skip)]
    colour: Colour,
}

impl DoomFlicker {
    pub fn new(address: LedCode, colour: Colour, max_percentage: u8, min_percentage: u8) -> Self {
        Self {
            led: address,
            colour,
            count: 4,
            max_percentage,
            min_percentage,
            start_colour: colour,
        }
    }
}

impl EffectState for DoomFlicker {
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
            max_percentage,
            min_percentage,
            colour,
            start_colour,
            ..
        } = self;

        if self.count == 0 {
            self.count = 4;
        }
        self.count -= 1;
        if self.count != 0 {
            return;
        }

        // TODO: make a "percentage" method on Colour.
        let (max_light, min_light) =
            compute_light_range(*start_colour, *max_percentage, *min_percentage);

        // Convert the 255 to percentage
        let amount = (p_random() & 7) as f32 * 8.0;

        let set_colour = |colour: &mut u8, max: f32, min: f32| {
            let pc = amount / max * 100.0;
            let min_amount = pc * min / 100.0; // percentage of min colour
            let max_amount = pc * max / 100.0; // percentage of max colour
            if *colour as f32 - min_amount < min {
                *colour = min as u8;
            } else {
                *colour = (max - max_amount) as u8;
            }
        };
        set_colour(&mut colour.r, max_light.r as f32, min_light.r as f32);
        set_colour(&mut colour.g, max_light.g as f32, min_light.g as f32);
        set_colour(&mut colour.b, max_light.b as f32, min_light.b as f32);

        self.count = 4;
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DoomLightFlash {
    led: LedCode,
    start_colour: Colour,
    max_percentage: u8,
    min_percentage: u8,
    #[serde(skip)]
    max_time: i32,
    #[serde(skip)]
    min_time: i32,
    #[serde(skip)]
    count: u8,
    #[serde(skip)]
    colour: Colour,
}

impl DoomLightFlash {
    pub fn new(address: LedCode, colour: Colour, max_percentage: u8, min_percentage: u8) -> Self {
        Self {
            led: address,
            colour,
            count: 4,
            max_percentage,
            min_percentage,
            start_colour: colour,
            max_time: 32,
            min_time: 7,
        }
    }
}

impl EffectState for DoomLightFlash {
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
            max_percentage,
            min_percentage,
            colour,
            start_colour,
            ..
        } = self;

        self.count -= 1;
        if self.count != 0 {
            return;
        }

        // TODO: make a "percentage" method on Colour.
        let (max_light, min_light) =
            compute_light_range(*start_colour, *max_percentage, *min_percentage);

        if *colour == max_light {
            *colour = min_light;
            self.count = ((p_random() & self.min_time) + 1) as u8;
        } else {
            *colour = max_light;
            self.count = ((p_random() & self.max_time) + 1) as u8;
        }
    }
}
