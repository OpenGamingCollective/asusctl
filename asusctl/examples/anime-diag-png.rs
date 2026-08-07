use std::env;
use std::error::Error;
use std::path::Path;
use std::process::exit;

use rog_anime::usb::get_anime_type;
use rog_anime::{AnimeDiagonal, AnimeType};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::system().unwrap();
    let proxy = AnimeProxyBlocking::new(&conn).unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: <filepath> <brightness>");
        println!("e.g, asusctl/examples/doom_large.png 0.8");
        exit(-1);
    }

    let matrix = AnimeDiagonal::from_png(
        Path::new(&args[1]),
        args[2].parse::<f32>().unwrap(),
        AnimeType::GA401,
    )?;

    let anime_type = get_anime_type();

    proxy.write(matrix.into_data_buffer(anime_type)?).unwrap();

    Ok(())
}
