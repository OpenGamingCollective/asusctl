use std::convert::TryFrom;
use std::path::Path;

pub use glam::Vec2;
use glam::{Mat3, Vec3};
use log::error;

use crate::AnimeType;
use crate::data::AnimeDataBuffer;
use crate::error::{AnimeError, Result};

/// A single greyscale + alpha pixel in the image
#[derive(Copy, Clone, Debug)]
pub struct Pixel {
    pub color: u32,
    pub alpha: f32,
}

impl Default for Pixel {
    #[inline]
    fn default() -> Self {
        Pixel {
            color: 0,
            alpha: 0.0,
        }
    }
}

/// A single LED position and brightness. The intention of this struct
/// is to be used to sample an image and set the LED brightness.
///
/// The position of the Led in `LedPositions` determines the placement in the
/// final data packets when written to the `AniMe`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Led(f32, f32, u8);

impl Led {
    const fn new(x: f32, y: f32) -> Self {
        Led(x, y, 0)
    }

    pub(crate) const fn x(&self) -> f32 {
        self.0
    }

    pub(crate) const fn y(&self) -> f32 {
        self.1
    }

    const fn bright(&self) -> u8 {
        self.2
    }

    fn set_bright(&mut self, a: u8) {
        self.2 = a;
    }
}

/// Container of `Led`, each of which specifies a position within the image
/// The main use of this is to position and sample colours for the final image
/// to show on `AniMe`
pub struct AnimeImage {
    pub scale: Vec2,
    /// Angle in radians
    pub angle: f32,
    /// Position of the image ont he display
    pub translation: Vec2,
    /// Brightness of final image, `0.0` = off, `1.0` = full
    pub bright: f32,
    /// Positions of all the LEDs
    led_pos: Vec<Option<Led>>,
    /// THe image data for sampling
    img_pixels: Vec<Pixel>,
    /// width of the image
    width: u32,
    /// The type of the display. The GA401 and GA402 use the same controller and
    /// therefore same ID, so the identifier must be by laptop model in
    /// `AnimeType`.
    anime_type: AnimeType,
}

impl AnimeImage {
    /// Exposed only for tests. Please use `from_png()` instead.
    pub fn new(
        scale: Vec2,
        angle: f32,
        translation: Vec2,
        bright: f32,
        pixels: Vec<Pixel>,
        width: u32,
        anime_type: AnimeType,
    ) -> Result<Self> {
        if !(0.0..=1.0).contains(&bright) {
            return Err(AnimeError::InvalidBrightness(bright));
        }

        Ok(Self {
            scale,
            angle,
            translation,
            bright,
            led_pos: Self::generate_image_positioning(anime_type),
            img_pixels: pixels,
            width,
            anime_type,
        })
    }

    // TODO: Convert functions back to const after todo completed

    /// Scale ratio in CM
    ///
    /// This is worked out by measuring the physical width of the display from
    /// pixel center to center, then dividing by `<horizontal LED count> +
    /// 0.5`, where the LED count is first/longest row.
    ///
    /// For GA401 this is `26.8 / (33 + 0.5) = 0.8`
    /// For GA402 this is `27.4 / (35 + 0.5) = 0.77`
    /// For GA402 this is `30.9 / (39 + 0.5) = 0.77`
    fn scale_x(anime_type: AnimeType) -> f32 {
        match anime_type {
            AnimeType::GA401 => 0.8,
            AnimeType::GU604 => 0.78,
            _ => 0.77,
        }
    }

    /// Scale ratio in CM
    ///
    /// This is worked out by measuring the physical height of the display from
    /// pixel center to pixel center, then dividing by 10 divided  `<vertical
    /// LED count> + 1.0`, where the LED count is first/longest row.
    ///
    /// For GA401 this is `16.5 / (54.0 + 1.0) = 0.3`
    /// For GA402 this is `17.3 / (61.0)       = 0.283`
    /// For GU604 this is `17.7 / (62.0 + 1)   = 0.28`
    fn scale_y(anime_type: AnimeType) -> f32 {
        match anime_type {
            AnimeType::GA401 => 0.3,
            AnimeType::GA402 => 0.283,
            _ => 0.28,
        }
    }

    /// Get the starting X position for the data we actually require when
    /// writing it out to LEDs.
    ///
    /// In relation to the display itself you should think of it as a full
    /// square grid, so `first_x` is the x position on that grid where the
    /// LED is actually positioned in relation to the Y.
    ///
    /// For GA401/GA402/GU604 (shrinking pattern - diagonal cuts in from left):
    /// ```text
    /// +------------+
    /// |            |
    /// |            |
    ///  \           |
    ///   \          |
    ///    \         |
    ///     \        |
    ///      \       |
    /// |----|\      |
    ///    ^   ------+
    ///  first_x
    /// ```
    ///
    /// For G835L/G635L (inverted pattern - triangle grows then rectangle shifts):
    /// ```text
    /// ●              <- Row 0: first_x = 0, width = 1
    ///  ●             <- Row 1: first_x = 0 (stagger), width = 1
    /// ● ●            <- Row 2: first_x = 0, width = 2
    ///  ● ●           <- Row 3: first_x = 0 (stagger), width = 2
    /// ...            <- Triangle continues, first_x = 0 for rows 0-27
    ///       ● ● ● ● ● ● ● ● ● ● ● ● ● ● ●  <- Row 28+: first_x grows
    ///        ● ● ● ● ● ● ● ● ● ● ● ● ● ● ● <- Rectangle shifts right
    /// ```
    /// Triangle (rows 0-27): first_x = 0 (no cumulative shift)
    /// Rectangle (rows 28-67): first_x = (y - 28) / 2 (shifts right)
    fn first_x(anime_type: AnimeType, y: u32) -> u32 {
        match anime_type {
            AnimeType::GA401 => {
                if y < 5 {
                    // first 5 rows for GA401 are always at X = 0
                    return 0;
                }
                y.div_ceil(2) - 3
            }
            AnimeType::GU604 => {
                // first 9 rows start at zero
                if y <= 9 {
                    return 0;
                }
                // and then their offset grows by one every two rows
                (y - 9) / 2
            }
            AnimeType::G635L | AnimeType::G835L => {
                // G835L/G635L have inverted geometry - triangle at top-left, rectangle shifts right
                // Triangle (rows 0-27): no cumulative shift, just alternating stagger
                // Rectangle (rows 28-67): shifts right by ~0.5px per row
                if y < 28 { 0 } else { (y - 28) / 2 }
            }
            _ => {
                // first 11 rows start at zero
                if y <= 11 {
                    return 0;
                }
                // and then their offset grows by one every two rows
                y.div_ceil(2) - 5
            }
        }
    }

    /// Width in LED count
    ///
    /// This is how many LED's are physically in a row
    /// ```text
    /// +------------+
    /// |            |
    /// |            |
    ///  \           |
    ///   \   width  |
    ///    \    v    |
    ///     \|------||
    ///      \       |
    ///       \      |
    ///        ------+
    /// ```
    // TODO: make this return only width, and move calcs to pitch
    fn width(anime_type: AnimeType, y: u32) -> u32 {
        match anime_type {
            AnimeType::GA401 => {
                if y < 5 {
                    // First 5 rows for GA401 are always 33 physical LEDs long
                    return 33;
                }
                36 - y.div_ceil(2)
            }
            AnimeType::GU604 => {
                if y <= 9 {
                    return 38 + y % 2;
                }
                38 - Self::first_x(anime_type, y) + y % 2
            }
            AnimeType::G635L | AnimeType::G835L => {
                // G835L/G635L rows GROW then stay constant (inverted from other devices)
                // Triangle (rows 0-27): pairs of rows with same length, 1→14
                // Rectangle (rows 28-67): constant 15 LEDs
                if y < 28 { y / 2 + 1 } else { 15 }
            }
            _ => {
                if y <= 11 {
                    return 34;
                }
                39 - y / 2
            }
        }
    }

    /// Physical display width by count of LED
    fn phys_width(anime_type: AnimeType) -> f32 {
        match anime_type {
            // 33.0 = Longest row LED count (physical) plus half-pixel offset
            AnimeType::GA401 => (33.0 + 0.5) * Self::scale_x(anime_type),
            AnimeType::GU604 => (38.0 + 0.5) * Self::scale_x(anime_type),
            AnimeType::G635L | AnimeType::G835L => (33.0 + 0.5) * Self::scale_x(anime_type),
            _ => (35.0 + 0.5) * Self::scale_x(anime_type),
        }
    }

    /// Height in LED count of longest column (physical count)
    fn height(anime_type: AnimeType) -> u32 {
        match anime_type {
            AnimeType::GA401 => 55,
            AnimeType::GU604 => 62,
            AnimeType::G635L | AnimeType::G835L => 68,
            _ => 61,
        }
    }

    /// Physical display height
    fn phys_height(anime_type: AnimeType) -> f32 {
        match anime_type {
            // 54.0 = End column LED count (physical) plus one dead pixel
            AnimeType::GA401 => (54.0 + 1.0) * Self::scale_y(anime_type),
            AnimeType::GU604 => 62.0 * Self::scale_y(anime_type),
            AnimeType::G635L | AnimeType::G835L => 68.0 * Self::scale_y(anime_type),
            // GA402 may not have dead pixels and require only the physical LED count
            _ => 61.0 * Self::scale_y(anime_type),
        }
    }

    /// Find the actual width of the data including the dead pixels
    fn pitch(anime_type: AnimeType, y: u32) -> u32 {
        match anime_type {
            AnimeType::GA401 => match y {
                0 | 2 | 4 => 33,
                1 | 3 => 35, // Some rows are padded
                _ => 36 - y / 2,
            },

            // Other devices don't have dead pixels
            _ => AnimeImage::width(anime_type, y),
        }
    }

    pub(crate) fn get_mut(&mut self) -> &mut [Pixel] {
        &mut self.img_pixels
    }

    /// Generate a list of LED positions. These are then used to sample the
    /// Image data, and will contain their resulting brightness.
    #[inline]
    pub fn generate_image_positioning(anime_type: AnimeType) -> Vec<Option<Led>> {
        (0..AnimeImage::height(anime_type))
            .flat_map(|y| {
                // For each row (Y) get actual length
                (0..AnimeImage::pitch(anime_type, y)).map(move |l| {
                    if l < AnimeImage::width(anime_type, y) {
                        let x = AnimeImage::first_x(anime_type, y) + l;
                        Some(Led::new(x as f32 - 0.5 * (y % 2) as f32, y as f32))
                    } else {
                        None // dead/non-existent pixels to the left
                    }
                })
            })
            .collect()
    }

    /// Called after setting new angle, position, or scale to refresh the image
    /// samples, the result can then been transformed to the appropriate data
    /// for displaying.
    ///
    /// The internal for loop iterates over the LED positions, skipping the
    /// blank/dead pixels if any.
    #[inline]
    pub fn update(&mut self) {
        let width = self.width as i32;
        let height = self.img_pixels.len() as i32 / width;
        let led_from_px = self.put(width as f32, height as f32);
        // Steps should be configurable as "sharpness"
        let du = led_from_px * Vec3::new(-0.5, 0.5, 0.0);
        let dv = led_from_px * Vec3::new(0.5, 0.5, 0.0);

        for led in self.led_pos.iter_mut().flatten() {
            let mut sum = 0.0;
            let mut alpha = 0.0;
            let mut count = 0;

            let pos = Vec3::new(led.x(), led.y(), 1.0);
            let x0 = led_from_px.mul_vec3(pos + Vec3::new(0.0, -0.5, 0.0));

            const GROUP: [f32; 4] = [
                0.0, 0.5, 1.0, 1.5,
            ];
            for u in &GROUP {
                for v in &GROUP {
                    let sample = x0 + *u * du + *v * dv;

                    let x = sample.x as i32;
                    let y = sample.y as i32;
                    if x > width - 1 || y > height - 1 || x < 0 || y < 0 {
                        continue;
                    }

                    let p = self.img_pixels[(x + (y * width)) as usize];
                    sum += p.color as f32;
                    alpha += p.alpha;
                    count += 1;
                }
            }
            alpha /= count as f32;
            sum /= count as f32;
            led.set_bright((sum * self.bright * alpha) as u8);
        }
    }

    /// A helper for determining physical position alignment
    pub fn edge_outline(&mut self) {
        // Janky shit here just to try help align images
        let mut last_x = 0.0;
        let mut last_y = 0.0;
        let mut last_was_led = false;
        let mut ends = Vec::new();
        for (idx, led) in self.led_pos.iter_mut().enumerate() {
            if let Some(led) = led {
                // Capture the starting LED
                if led.x() - last_x != 1.0 {
                    led.set_bright(255);
                    last_x = led.x();
                } else {
                    // top and bottom
                    if led.y() == 0.0 || led.y() >= AnimeImage::height(self.anime_type) as f32 - 1.0
                    {
                        led.set_bright(255);
                    }
                    last_x += 1.0;
                }
                if led.y() - last_y == 1.0 {
                    ends.push(idx);
                    last_y = led.y();
                }
                last_was_led = true;
            } else if last_was_led {
                // ends.push(idx);
                last_was_led = false;
            }
        }
        for end in ends {
            if let Some(led) = self.led_pos[end - 1].as_mut() {
                led.set_bright(255);
            }
        }
    }

    /// Put the render window in place on the image
    fn put(&self, bmp_w: f32, bmp_h: f32) -> Mat3 {
        // Center of image
        let center = Mat3::from_translation(Vec2::new(-0.5 * bmp_w, -0.5 * bmp_h));
        // Find the scale required for cleanly showing the image
        let h = AnimeImage::phys_height(self.anime_type) / bmp_h;
        let mut base_scale = AnimeImage::phys_width(self.anime_type) / bmp_w;
        if base_scale > h {
            base_scale = h;
        }

        let cm_from_px = Mat3::from_scale(Vec2::new(base_scale, base_scale));

        let led_from_cm = Mat3::from_scale(Vec2::new(
            1.0 / AnimeImage::scale_x(self.anime_type),
            1.0 / AnimeImage::scale_y(self.anime_type),
        ));

        let transform =
            Mat3::from_scale_angle_translation(self.scale, self.angle, self.translation);

        let pos_in_leds = Mat3::from_translation(self.led_center());
        // Get LED-to-image coords
        let led_from_px = pos_in_leds * led_from_cm * transform * cm_from_px * center;

        led_from_px.inverse()
    }

    fn led_center(&self) -> Vec2 {
        if !matches!(self.anime_type, AnimeType::G635L | AnimeType::G835L) {
            return Vec2::new(20.0, 20.0);
        }

        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for led in self.led_pos.iter().flatten() {
            let pos = Vec2::new(led.x(), led.y());
            min = min.min(pos);
            max = max.max(pos);
        }

        if min.x.is_finite() {
            let mut center = (min + max) * 0.5;
            center.y += 1.0;
            center
        } else {
            Vec2::new(20.0, 20.0)
        }
    }

    /// Generate the base image from inputs. The result can be displayed as is
    /// or updated via scale, position, or angle then displayed again after
    /// `update()`.
    #[inline]
    pub fn from_png(
        path: &Path,
        scale: f32,
        angle: f32,
        translation: Vec2,
        bright: f32,
        anime_type: AnimeType,
    ) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| {
            error!("Could not open {path:?}: {e:?}");
            e
        })?;
        let data = std::io::Cursor::new(data);
        let decoder = png_pong::Decoder::new(data)?.into_steps();
        let png_pong::Step { raster, delay: _ } = decoder.last().ok_or(AnimeError::NoFrames)??;

        let width;
        let pixels = match &raster {
            png_pong::PngRaster::Gray8(ras) => {
                width = ras.width();
                Self::pixels_from_8bit(ras, true)
            }
            png_pong::PngRaster::Graya8(ras) => {
                width = ras.width();
                Self::pixels_from_8bit(ras, true)
            }
            png_pong::PngRaster::Rgb8(ras) => {
                width = ras.width();
                Self::pixels_from_8bit(ras, false)
            }
            png_pong::PngRaster::Rgba8(ras) => {
                width = ras.width();
                Self::pixels_from_8bit(ras, false)
            }
            png_pong::PngRaster::Gray16(ras) => {
                width = ras.width();
                Self::pixels_from_16bit(ras, true)
            }
            png_pong::PngRaster::Rgb16(ras) => {
                width = ras.width();
                Self::pixels_from_16bit(ras, false)
            }
            png_pong::PngRaster::Graya16(ras) => {
                width = ras.width();
                Self::pixels_from_16bit(ras, true)
            }
            png_pong::PngRaster::Rgba16(ras) => {
                width = ras.width();
                Self::pixels_from_16bit(ras, false)
            }
            png_pong::PngRaster::Palette(..) => return Err(AnimeError::Format),
        };

        let mut matrix = AnimeImage::new(
            Vec2::new(scale, scale),
            angle,
            translation,
            bright,
            pixels,
            width,
            anime_type,
        )?;

        matrix.update();
        Ok(matrix)
    }

    fn pixels_from_8bit<P>(ras: &pix::Raster<P>, grey: bool) -> Vec<Pixel>
    where
        P: pix::el::Pixel<Chan = pix::chan::Ch8>,
    {
        ras.pixels()
            .iter()
            .map(|px| crate::image::Pixel {
                color: if grey {
                    <u8>::from(px.one()) as u32
                } else {
                    (<u8>::from(px.one()) / 3) as u32
                        + (<u8>::from(px.two()) / 3) as u32
                        + (<u8>::from(px.three()) / 3) as u32
                },
                alpha: <f32>::from(px.alpha()),
            })
            .collect()
    }

    fn pixels_from_16bit<P>(ras: &pix::Raster<P>, grey: bool) -> Vec<Pixel>
    where
        P: pix::el::Pixel<Chan = pix::chan::Ch16>,
    {
        ras.pixels()
            .iter()
            .map(|px| crate::image::Pixel {
                color: if grey {
                    (<u16>::from(px.one()) >> 8) as u32
                } else {
                    ((<u16>::from(px.one()) / 3) >> 8) as u32
                        + ((<u16>::from(px.two()) / 3) >> 8) as u32
                        + ((<u16>::from(px.three()) / 3) >> 8) as u32
                },
                alpha: <f32>::from(px.alpha()),
            })
            .collect()
    }
}

impl TryFrom<&AnimeImage> for AnimeDataBuffer {
    type Error = AnimeError;

    /// Do conversion from the nested Vec in `AnimeDataBuffer` to the two
    /// required packets suitable for sending over USB
    fn try_from(leds: &AnimeImage) -> Result<Self> {
        let mut l: Vec<u8> = leds
            .led_pos
            .iter()
            .map(|l| if let Some(l) = l { l.bright() } else { 0 })
            .collect();
        let mut v = Vec::with_capacity(leds.anime_type.data_length());
        if leds.anime_type == AnimeType::GA401 {
            v.push(0);
        }
        v.append(&mut l);
        v.append(&mut vec![0u8; leds.anime_type.data_length() - v.len()]);
        AnimeDataBuffer::from_vec(leds.anime_type, v)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::path::PathBuf;

    use crate::image::*;
    use crate::{AnimTime, AnimeGif, AnimePacketType};

    #[test]
    fn led_positions() {
        let leds = AnimeImage::generate_image_positioning(AnimeType::GA401);
        assert_eq!(leds[0], Some(Led(0.0, 0.0, 0)));
        assert_eq!(leds[1], Some(Led(1.0, 0.0, 0)));
        assert_eq!(leds[2], Some(Led(2.0, 0.0, 0)));
        assert_eq!(leds[32], Some(Led(32.0, 0.0, 0)));
        assert_eq!(leds[33], Some(Led(-0.5, 1.0, 0)));
        assert_eq!(leds[65], Some(Led(31.5, 1.0, 0)));
        assert_eq!(leds[66], None);
        assert_eq!(leds[67], None);
        assert_eq!(leds[68], Some(Led(0.0, 2.0, 0)));
        assert_eq!(leds[100], Some(Led(32.0, 2.0, 0)));
        assert_eq!(leds[101], Some(Led(-0.5, 3.0, 0)));
        assert_eq!(leds[133], Some(Led(31.5, 3.0, 0)));
        assert_eq!(leds[134], None);
        assert_eq!(leds[135], None);
        assert_eq!(leds[136], Some(Led(0.0, 4.0, 0)));
        assert_eq!(leds[168], Some(Led(32.0, 4.0, 0)));
        assert_eq!(leds[169], Some(Led(-0.5, 5.0, 0)));
        assert_eq!(leds[201], Some(Led(31.5, 5.0, 0)));
        assert_eq!(leds[202], None);
        assert_eq!(leds[203], Some(Led(0.0, 6.0, 0)));
        assert_eq!(leds[235], Some(Led(32.0, 6.0, 0)));
        assert_eq!(leds[648], Some(Led(32.0, 20.0, 0))); // end
        assert_eq!(leds[649], Some(Led(7.5, 21.0, 0))); // start
        assert_eq!(leds[673], Some(Led(31.5, 21.0, 0))); // end
    }

    // #[test]
    // fn led_positions_const() {
    //     let leds = AnimeImage::generate();
    //     assert_eq!(leds[1], LED_IMAGE_POSITIONS[1]);
    //     assert_eq!(leds[34], LED_IMAGE_POSITIONS[34]);
    //     assert_eq!(leds[69], LED_IMAGE_POSITIONS[69]);
    //     assert_eq!(leds[137], LED_IMAGE_POSITIONS[137]);
    //     assert_eq!(leds[169], LED_IMAGE_POSITIONS[169]);
    //     assert_eq!(leds[170], LED_IMAGE_POSITIONS[170]);
    //     assert_eq!(leds[236], LED_IMAGE_POSITIONS[236]);
    //     assert_eq!(leds[649], LED_IMAGE_POSITIONS[649]);
    //     assert_eq!(leds[674], LED_IMAGE_POSITIONS[674]);
    // }

    #[test]
    fn row_starts() {
        let a = AnimeType::GA401;
        assert_eq!(AnimeImage::first_x(a, 5), 0);
        assert_eq!(AnimeImage::first_x(a, 6), 0);
        assert_eq!(AnimeImage::first_x(a, 7), 1);
        assert_eq!(AnimeImage::first_x(a, 8), 1);
        assert_eq!(AnimeImage::first_x(a, 9), 2);
        assert_eq!(AnimeImage::first_x(a, 10), 2);
        assert_eq!(AnimeImage::first_x(a, 11), 3);
    }

    #[test]
    fn row_widths() {
        let a = AnimeType::GA401;
        assert_eq!(AnimeImage::width(a, 5), 33);
        assert_eq!(AnimeImage::width(a, 6), 33);
        assert_eq!(AnimeImage::width(a, 7), 32);
        assert_eq!(AnimeImage::width(a, 8), 32);
        assert_eq!(AnimeImage::width(a, 9), 31);
        assert_eq!(AnimeImage::width(a, 10), 31);
        assert_eq!(AnimeImage::width(a, 11), 30);
        assert_eq!(AnimeImage::width(a, 12), 30);
        assert_eq!(AnimeImage::width(a, 13), 29);
        assert_eq!(AnimeImage::width(a, 14), 29);
        assert_eq!(AnimeImage::width(a, 15), 28);
        assert_eq!(AnimeImage::width(a, 16), 28);
        assert_eq!(AnimeImage::width(a, 17), 27);
        assert_eq!(AnimeImage::width(a, 18), 27);
    }

    #[test]
    fn row_pitch() {
        let a = AnimeType::GA401;
        assert_eq!(AnimeImage::pitch(a, 5), 34);
        assert_eq!(AnimeImage::pitch(a, 6), 33);
        assert_eq!(AnimeImage::pitch(a, 7), 33);
        assert_eq!(AnimeImage::pitch(a, 8), 32);
        assert_eq!(AnimeImage::pitch(a, 9), 32);
        assert_eq!(AnimeImage::pitch(a, 10), 31);
        assert_eq!(AnimeImage::pitch(a, 11), 31);
        assert_eq!(AnimeImage::pitch(a, 12), 30);
        assert_eq!(AnimeImage::pitch(a, 13), 30);
        assert_eq!(AnimeImage::pitch(a, 14), 29);
    }

    #[test]
    #[ignore = "Just to inspect image packet"]
    fn ga402_image_packet_check() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/anime/custom/sonic-run.gif");

        let matrix = AnimeGif::from_gif(
            &path,
            1.0,
            0.0,
            Vec2::default(),
            AnimTime::Infinite,
            1.0,
            AnimeType::GA402,
        )
        .unwrap();
        matrix.frames()[0].frame();
        let _pkt = AnimePacketType::try_from(matrix.frames()[0].frame().clone()).unwrap();
    }
}
