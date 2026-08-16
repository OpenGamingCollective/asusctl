//! Using a combination of key-colour array plus a key layout to generate
//! outputs.

use rog_aura::Colour;
use rog_aura::effects::{AdvancedEffects, Effect};
use rog_aura::keyboard::{KeyLayout, LedCode};
use rog_dbus::zbus_aura::AuraProxyBlocking;
use zbus::blocking::Connection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layout = KeyLayout::default_layout();

    let conn = Connection::system().unwrap();
    let proxy = AuraProxyBlocking::new(&conn).unwrap();

    let mut seq = AdvancedEffects::new(true);

    // let zone = Effect::Breathe(rog_aura::effects::Breathe::new(
    //     RgbAddress::Single,
    //     Colour(166, 127, 166),
    //     Colour(127, 155, 20),
    //     rog_aura::Speed::High,
    // ));
    // seq.push(zone);

    // let zone = Effect::DoomLightFlash(rog_aura::effects::DoomLightFlash::new(
    //     RgbAddress::Single,
    //     Colour(200, 0, 0),
    //     80,
    //     10,
    // ));
    // seq.push(zone);

    let zone = Effect::DoomFlicker(rog_aura::effects::DoomFlicker::new(
        LedCode::SingleZone,
        Colour {
            r: 200,
            g: 110,
            b: 0,
        },
        100,
        10,
    ));
    seq.push(zone);

    // let zone = Effect::Breathe(rog_aura::effects::Breathe::new(
    //     RgbAddress::KeyboardCenterLeft,
    //     Colour(16, 127, 255),
    //     Colour(127, 15, 20),
    //     rog_aura::Speed::Low,
    // ));
    // seq.push(zone);

    // let zone = Effect::Breathe(rog_aura::effects::Breathe::new(
    //     RgbAddress::LightbarRightCorner,
    //     Colour(0, 255, 255),
    //     Colour(255, 0, 255),
    //     rog_aura::Speed::Med,
    // ));
    // seq.push(zone);

    loop {
        seq.next_state(&layout);
        let packets = seq.create_packets();

        proxy.direct_addressing_raw(packets)?;
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
}
