use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::f32::consts::PI;
use std::path::Path;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use rog_anime::usb::get_anime_type;
use rog_anime::{AnimeDataBuffer, AnimeImage, Vec2};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::system().unwrap();
    let proxy = AnimeProxyBlocking::new(&conn).unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() != 7 {
        println!("Usage: <filepath> <scale> <angle> <x pos> <y pos> <brightness>");
        println!("e.g, asusctl/examples/doom_large.png 0.9 0.4 0.0 0.0 0.8");
        exit(-1);
    }

    let anime_type = get_anime_type();
    let mut matrix = AnimeImage::from_png(
        Path::new(&args[1]),
        args[2].parse::<f32>().unwrap(),
        args[3].parse::<f32>().unwrap(),
        Vec2::new(
            args[4].parse::<f32>().unwrap(),
            args[5].parse::<f32>().unwrap(),
        ),
        args[6].parse::<f32>().unwrap(),
        anime_type,
    )?;

    loop {
        matrix.angle += 0.05;
        if matrix.angle > PI * 2.0 {
            matrix.angle = 0.0;
        }
        matrix.update();

        proxy.write(&<AnimeDataBuffer>::try_from(&matrix)?).unwrap();
        sleep(Duration::from_micros(500));
    }
}
