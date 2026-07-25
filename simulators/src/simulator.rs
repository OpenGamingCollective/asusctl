use std::env;
use std::error::Error;
use std::str::FromStr;

use rog_anime::usb::{PROD_ID, VENDOR_ID};
use rog_anime::{AnimeType, USB_PREFIX2};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use uhid_virt::{Bus, CreateParams, UHIDDevice};

mod animatrix;
use animatrix::*;

pub struct VirtAnimeMatrix {
    device: UHIDDevice<std::fs::File>,
    buffer: [u8; 640],
    animatrix: AniMatrix,
}

// TODO: This isn't working
impl VirtAnimeMatrix {
    pub fn new(model: AnimeType) -> Self {
        VirtAnimeMatrix {
            buffer: [0; 640],
            animatrix: AniMatrix::new(model),
            device: UHIDDevice::create(CreateParams {
                name: String::from("ROG_Virtual Anime Matrix"),
                phys: String::from(""),
                uniq: String::from(""),
                bus: Bus::USB,
                vendor: VENDOR_ID as u32,
                product: PROD_ID as u32,
                version: 0,
                country: 0,
                // This is a device which emits the usage code as a whole, rather than as bits
                rd_data: [
                    0x06, 0x31, 0xff, // Usage Page (Vendor Defined 0xFF31)
                    0x09, 0x76, // Usage (0x76)
                    0xa1, 0x01, // Collection (Application)
                    0x85, 0x5a, //   Report ID (90)
                    0x19, 0x00, //   Usage Minimum (0x00)
                    0x2a, 0xff, 0x00, //   Usage Maximum (0xFF)
                    0x15, 0x00, //   Logical Minimum (0)
                    0x26, 0xff, 0x00, //   Logical Maximum (255)
                    0x75, 0x08, //   Report Size (8)
                    0x95, 0x05, //   Report Count (5)
                    0x81,
                    0x00, /*   Input (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null
                           * Position) */
                    0x19, 0x00, //   Usage Minimum (0x00)
                    0x2a, 0xff, 0x00, //   Usage Maximum (0xFF)
                    0x15, 0x00, //   Logical Minimum (0)
                    0x26, 0xff, 0x00, //   Logical Maximum (255)
                    0x75, 0x08, //   Report Size (8)
                    0x95, 0x3f, //   Report Count (63)
                    0xb1,
                    0x00, /*   Feature (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null
                           * Position,Non-volatile) */
                    0xc0, // End Collection
                    0x06, 0x31, 0xff, // Usage Page (Vendor Defined 0xFF31)
                    0x09, 0x80, // Usage (0x80)
                    0xa1, 0x01, // Collection (Application)
                    0x85, 0x5e, //   Report ID (94)
                    0x19, 0x00, //   Usage Minimum (0x00)
                    0x2a, 0xff, 0x00, //   Usage Maximum (0xFF)
                    0x15, 0x00, //   Logical Minimum (0)
                    0x26, 0xff, 0x00, //   Logical Maximum (255)
                    0x75, 0x08, //   Report Size (8)
                    0x95, 0x05, //   Report Count (5)
                    0x81,
                    0x00, /*   Input (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null
                           * Position) */
                    0x19, 0x00, //   Usage Minimum (0x00)
                    0x2a, 0xff, 0x00, //   Usage Maximum (0xFF)
                    0x15, 0x00, //   Logical Minimum (0)
                    0x26, 0xff, 0x00, //   Logical Maximum (255)
                    0x96, 0x7f, 0x02, //   Report Count (639)
                    0xb1,
                    0x00, /*   Feature (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null
                           * Position,Non-volatile) */
                    0xc0, /* End Collection */

                          /* 85 bytes */
                ]
                .to_vec(),
            })
            .unwrap_or_else(|err| {
                panic!(
                    "Could not create virtual device: {err:?}. \
                    Try loading the uhid module and ensure you have the necessary permissions."
                )
            }),
        }
    }

    // /// A single on/off key press
    // pub fn press(&mut self, input: [u8; 32]) {
    //     self.device.write(&input).unwrap();
    //     let mut reset = [0u8; 32];
    //     reset[0] = input[0];
    //     self.device.write(&reset).unwrap();
    // }

    pub fn read(&mut self) {
        if let Ok(uhid_virt::OutputEvent::Output { data }) = self.device.read() {
            for (i, b) in self.buffer.iter_mut().enumerate() {
                *b = 0;
                if let Some(n) = data.get(i) {
                    *b = *n;
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("Must supply arg, one of <GA401, GA402, GU604, G835L>");
        return Ok(());
    }
    let anime_type = AnimeType::from_str(&args[1])?;

    let sdl_context = sdl2::init().map_err(|e| e.to_string())?;
    let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

    let window = video_subsystem
        .window("rust-sdl2 demo", 1260, 760)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let mut dev = VirtAnimeMatrix::new(anime_type);

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    let mut event_pump = sdl_context.event_pump().map_err(|e| e.to_string())?;
    'running: loop {
        dev.read(); // it's blocking, and damned hard to sync with arc/mutex
                    // let one = dev.buffer[0..7] != USB_PREFIX2;
        let index = dev.buffer[3];

        let w = dev.animatrix.led_shape().horizontal * 6;
        let h = dev.animatrix.led_shape().vertical * 6;
        let mut y_offset = 0;
        for (y_count, row) in dev.animatrix.rows().iter().enumerate() {
            if row.0 == index {
                let start = row.1;
                let end = start + row.2;
                if row.1 < 10 && row.2 < 15 {
                    if index == 0x74 {
                        y_offset = 1;
                    } else if index == 0xe7 {
                        y_offset = 2;
                    }
                }
                for (x_count, b) in dev.buffer[start..=end].iter().enumerate() {
                    canvas.set_draw_color(Color::RGB(*b, *b, *b));

                    #[allow(clippy::manual_is_multiple_of)]
                    let x: i32 = w + x_count as i32 * w
                        - if (y_count + y_offset as usize) % 2 != 0 {
                            0
                        } else {
                            w / 2
                        }
                        + row.3 * w;
                    let y = y_count as i32 * h - y_offset * h;
                    let _ = canvas.fill_rect(Rect::new(x, y, w as u32, h as u32));
                }
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        if dev.buffer[0..7] == USB_PREFIX2 {
            canvas.present();
        }
        // ::std::thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
