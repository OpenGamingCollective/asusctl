use std::env;
use std::path::Path;
use std::thread::sleep;

use rog_anime::usb::get_anime_type;
use rog_anime::{ActionData, ActionLoader, Sequences};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

fn main() {
    let conn = Connection::system().unwrap();
    let proxy = AnimeProxyBlocking::new(&conn).unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Please supply filepath and brightness");
        return;
    }

    let path = Path::new(&args[1]);
    let brightness = args[2].parse::<f32>().unwrap();
    let anime_type = get_anime_type();
    let mut seq = Sequences::new(anime_type);
    seq.insert(
        0,
        &ActionLoader::AsusAnimation {
            file: path.into(),
            time: rog_anime::AnimTime::Infinite,
            brightness,
        },
    )
    .unwrap();

    loop {
        for action in seq.iter() {
            if let ActionData::Animation(frames) = action {
                for frame in frames.frames() {
                    proxy.write(frame.frame()).unwrap();
                    sleep(frame.delay());
                }
            }
        }
    }
}
