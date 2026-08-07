use std::thread::sleep;
use std::time::Duration;

use rog_anime::usb::get_anime_type;
use rog_anime::{AnimeDiagonal, AnimeType};
use rog_dbus::zbus_anime::AnimeProxyBlocking;
use zbus::blocking::Connection;

// In usable data:
// Top row start at 1, ends at 32

// 74w x 36h diagonal used by the windows app

fn main() {
    let conn = Connection::system().unwrap();
    let proxy = AnimeProxyBlocking::new(&conn).unwrap();

    for step in (2..50).rev() {
        let mut matrix = AnimeDiagonal::new(AnimeType::GA401);
        for c in (0..60).step_by(step) {
            for i in matrix.get_mut().iter_mut() {
                i[c] = 50;
            }
        }

        for c in (0..35).step_by(step) {
            for i in &mut matrix.get_mut()[c] {
                *i = 50;
            }
        }

        let anime_type = get_anime_type();
        proxy
            .write(matrix.into_data_buffer(anime_type).unwrap())
            .unwrap();
        sleep(Duration::from_millis(300));
    }
}
