//! This is full of crap code which is basically bruteforced

use std::path::Path;
use std::time::Duration;

use log::error;

use crate::AnimeType;
use crate::data::AnimeDataBuffer;
use crate::error::{AnimeError, Result};

/// Mostly intended to be used with ASUS gifs, but can be used for other
/// purposes (like images)
#[allow(dead_code)]
pub struct AnimeDiagonal(AnimeType, Vec<Vec<u8>>, Option<Duration>);

impl AnimeDiagonal {
    #[inline]
    pub fn new(anime_type: AnimeType, duration: Option<Duration>) -> Self {
        Self(
            anime_type,
            vec![vec![0; anime_type.width()]; anime_type.height()],
            duration,
        )
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.1
    }

    /// Get a full diagonal row where `x` `y` is the starting point  and `len`
    /// is the length of data.
    fn get_row(&self, x: usize, y: usize, len: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(len);
        for i in 0..len {
            let y = self.0.height() - y - i - 1;
            let val = self.1[y][x + i];
            buf.push(val);
        }
        buf
    }

    /// Generate the base image from inputs. The result can be displayed as is
    /// or updated via scale, position, or angle then displayed again after
    /// `update()`.
    #[inline]
    pub fn from_png(
        path: &Path,
        duration: Option<Duration>,
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

        let mut matrix = AnimeDiagonal::new(anime_type, duration);

        match &raster {
            png_pong::PngRaster::Gray8(ras) => {
                Self::pixels_from_8bit(ras, &mut matrix, bright, true);
            }
            png_pong::PngRaster::Graya8(ras) => {
                Self::pixels_from_8bit(ras, &mut matrix, bright, true);
            }
            png_pong::PngRaster::Rgb8(ras) => {
                Self::pixels_from_8bit(ras, &mut matrix, bright, false);
            }
            png_pong::PngRaster::Rgba8(ras) => {
                Self::pixels_from_8bit(ras, &mut matrix, bright, false);
            }
            png_pong::PngRaster::Gray16(ras) => {
                Self::pixels_from_16bit(ras, &mut matrix, bright, true);
            }
            png_pong::PngRaster::Rgb16(ras) => {
                Self::pixels_from_16bit(ras, &mut matrix, bright, false);
            }
            png_pong::PngRaster::Graya16(ras) => {
                Self::pixels_from_16bit(ras, &mut matrix, bright, true);
            }
            png_pong::PngRaster::Rgba16(ras) => {
                Self::pixels_from_16bit(ras, &mut matrix, bright, false);
            }
            png_pong::PngRaster::Palette(..) => return Err(AnimeError::Format),
        };

        Ok(matrix)
    }

    fn pixels_from_8bit<P>(
        ras: &pix::Raster<P>,
        matrix: &mut AnimeDiagonal,
        bright: f32,
        grey: bool,
    ) where
        P: pix::el::Pixel<Chan = pix::chan::Ch8>,
    {
        let width = ras.width();
        for (y, row) in ras.pixels().chunks(width as usize).enumerate() {
            for (x, px) in row.iter().enumerate() {
                let v = if grey {
                    <u8>::from(px.one()) as f32
                } else {
                    (<u8>::from(px.one()) / 3) as f32
                        + (<u8>::from(px.two()) / 3) as f32
                        + (<u8>::from(px.three()) / 3) as f32
                };
                if y < matrix.1.len() && x < matrix.1[y].len() {
                    matrix.1[y][x] = (v * bright) as u8;
                }
            }
        }
    }

    fn pixels_from_16bit<P>(
        ras: &pix::Raster<P>,
        matrix: &mut AnimeDiagonal,
        bright: f32,
        grey: bool,
    ) where
        P: pix::el::Pixel<Chan = pix::chan::Ch16>,
    {
        let width = ras.width();
        for (y, row) in ras.pixels().chunks(width as usize).enumerate() {
            for (x, px) in row.iter().enumerate() {
                let v = if grey {
                    (<u16>::from(px.one()) >> 8) as f32
                } else {
                    ((<u16>::from(px.one()) / 3) >> 8) as f32
                        + ((<u16>::from(px.two()) / 3) >> 8) as f32
                        + ((<u16>::from(px.three()) / 3) >> 8) as f32
                };
                matrix.1[y][x] = (v * bright) as u8;
            }
        }
    }

    /// Convert to a data buffer that can be sent over dbus
    #[inline]
    pub fn into_data_buffer(&self, anime_type: AnimeType) -> Result<AnimeDataBuffer> {
        match anime_type {
            AnimeType::GA401 => self.to_ga401_packets(),
            AnimeType::GU604 => self.to_gu604_packets(),
            AnimeType::G635L | AnimeType::G835L => self.to_strix_packets(anime_type),
            _ => self.to_ga402_packets(),
        }
    }

    /// Do conversion from the nested Vec in `AnimeMatrix` to the two required
    /// packets suitable for sending over USB
    fn to_ga401_packets(&self) -> Result<AnimeDataBuffer> {
        let mut buf = vec![0u8; AnimeType::GA401.data_length()];

        buf[1..=32].copy_from_slice(&self.get_row(0, 3, 32));
        buf[34..=66].copy_from_slice(&self.get_row(0, 2, 33));
        buf[69..=101].copy_from_slice(&self.get_row(1, 2, 33)); // ?!
        buf[102..=134].copy_from_slice(&self.get_row(1, 1, 33));
        buf[137..=169].copy_from_slice(&self.get_row(2, 1, 33));
        buf[170..=202].copy_from_slice(&self.get_row(2, 0, 33));
        buf[204..=236].copy_from_slice(&self.get_row(3, 0, 33)); // This and above cause overflow?
        buf[237..=268].copy_from_slice(&self.get_row(4, 0, 32));
        buf[270..=301].copy_from_slice(&self.get_row(5, 0, 32));
        buf[302..=332].copy_from_slice(&self.get_row(6, 0, 31));
        buf[334..=364].copy_from_slice(&self.get_row(7, 0, 31));
        buf[365..=394].copy_from_slice(&self.get_row(8, 0, 30));
        buf[396..=425].copy_from_slice(&self.get_row(9, 0, 30));
        buf[426..=454].copy_from_slice(&self.get_row(10, 0, 29));
        buf[456..=484].copy_from_slice(&self.get_row(11, 0, 29));
        buf[485..=512].copy_from_slice(&self.get_row(12, 0, 28));
        buf[514..=541].copy_from_slice(&self.get_row(13, 0, 28));
        buf[542..=568].copy_from_slice(&self.get_row(14, 0, 27));
        buf[570..=596].copy_from_slice(&self.get_row(15, 0, 27));
        buf[597..=622].copy_from_slice(&self.get_row(16, 0, 26));
        buf[624..=649].copy_from_slice(&self.get_row(17, 0, 26));
        buf[650..=674].copy_from_slice(&self.get_row(18, 0, 25));
        buf[676..=700].copy_from_slice(&self.get_row(19, 0, 25));
        buf[701..=724].copy_from_slice(&self.get_row(20, 0, 24));
        buf[726..=749].copy_from_slice(&self.get_row(21, 0, 24));
        buf[750..=772].copy_from_slice(&self.get_row(22, 0, 23));
        buf[774..=796].copy_from_slice(&self.get_row(23, 0, 23));
        buf[797..=818].copy_from_slice(&self.get_row(24, 0, 22));
        buf[820..=841].copy_from_slice(&self.get_row(25, 0, 22));
        buf[842..=862].copy_from_slice(&self.get_row(26, 0, 21));
        buf[864..=884].copy_from_slice(&self.get_row(27, 0, 21));
        buf[885..=904].copy_from_slice(&self.get_row(28, 0, 20));
        buf[906..=925].copy_from_slice(&self.get_row(29, 0, 20));
        buf[926..=944].copy_from_slice(&self.get_row(30, 0, 19));
        buf[946..=964].copy_from_slice(&self.get_row(31, 0, 19));
        buf[965..=982].copy_from_slice(&self.get_row(32, 0, 18));
        buf[984..=1001].copy_from_slice(&self.get_row(33, 0, 18));
        buf[1002..=1018].copy_from_slice(&self.get_row(34, 0, 17));
        buf[1020..=1036].copy_from_slice(&self.get_row(35, 0, 17));
        buf[1037..=1052].copy_from_slice(&self.get_row(36, 0, 16));
        buf[1054..=1069].copy_from_slice(&self.get_row(37, 0, 16));
        buf[1070..=1084].copy_from_slice(&self.get_row(38, 0, 15));
        buf[1086..=1100].copy_from_slice(&self.get_row(39, 0, 15));
        buf[1101..=1114].copy_from_slice(&self.get_row(40, 0, 14));
        buf[1116..=1129].copy_from_slice(&self.get_row(41, 0, 14));
        buf[1130..=1142].copy_from_slice(&self.get_row(42, 0, 13));
        buf[1144..=1156].copy_from_slice(&self.get_row(43, 0, 13));
        buf[1157..=1168].copy_from_slice(&self.get_row(44, 0, 12));
        buf[1170..=1181].copy_from_slice(&self.get_row(45, 0, 12));
        buf[1182..=1192].copy_from_slice(&self.get_row(46, 0, 11));
        buf[1194..=1204].copy_from_slice(&self.get_row(47, 0, 11));
        buf[1205..=1214].copy_from_slice(&self.get_row(48, 0, 10));
        buf[1216..=1225].copy_from_slice(&self.get_row(49, 0, 10));
        buf[1226..=1234].copy_from_slice(&self.get_row(50, 0, 9));
        buf[1236..=1244].copy_from_slice(&self.get_row(51, 0, 9));

        AnimeDataBuffer::from_vec(crate::AnimeType::GA401, buf)
    }

    fn to_ga402_packets(&self) -> Result<AnimeDataBuffer> {
        let mut buf = vec![0u8; AnimeType::GA402.data_length()];
        let mut start_index: usize = 0;

        fn copy_slice(
            buf: &mut [u8],
            anime: &AnimeDiagonal,
            x: usize,
            y: usize,
            start_index: &mut usize,
            len: usize,
        ) {
            buf[*start_index..*start_index + len].copy_from_slice(&anime.get_row(x, y, len));
            *start_index += len;
        }

        let b = &mut buf;
        let a = &self;
        copy_slice(b, a, 0, 5, &mut start_index, 34);
        copy_slice(b, a, 1, 5, &mut start_index, 34);
        copy_slice(b, a, 1, 4, &mut start_index, 34);
        copy_slice(b, a, 2, 4, &mut start_index, 34);
        copy_slice(b, a, 2, 3, &mut start_index, 34);
        copy_slice(b, a, 3, 3, &mut start_index, 34);
        copy_slice(b, a, 3, 2, &mut start_index, 34);
        copy_slice(b, a, 4, 2, &mut start_index, 34);
        copy_slice(b, a, 4, 1, &mut start_index, 34);
        copy_slice(b, a, 5, 1, &mut start_index, 34);
        copy_slice(b, a, 5, 0, &mut start_index, 34);
        copy_slice(b, a, 6, 0, &mut start_index, 34);
        copy_slice(b, a, 7, 0, &mut start_index, 33);
        copy_slice(b, a, 8, 0, &mut start_index, 33);
        copy_slice(b, a, 9, 0, &mut start_index, 32);
        copy_slice(b, a, 10, 0, &mut start_index, 32);
        copy_slice(b, a, 11, 0, &mut start_index, 31);
        copy_slice(b, a, 12, 0, &mut start_index, 31);
        copy_slice(b, a, 13, 0, &mut start_index, 30);
        copy_slice(b, a, 14, 0, &mut start_index, 30);
        copy_slice(b, a, 15, 0, &mut start_index, 29);
        copy_slice(b, a, 16, 0, &mut start_index, 29);
        copy_slice(b, a, 17, 0, &mut start_index, 28);
        copy_slice(b, a, 18, 0, &mut start_index, 28);
        copy_slice(b, a, 19, 0, &mut start_index, 27);
        copy_slice(b, a, 20, 0, &mut start_index, 27);
        copy_slice(b, a, 21, 0, &mut start_index, 26);
        copy_slice(b, a, 22, 0, &mut start_index, 26);
        copy_slice(b, a, 23, 0, &mut start_index, 25);
        copy_slice(b, a, 24, 0, &mut start_index, 25);
        copy_slice(b, a, 25, 0, &mut start_index, 24);
        copy_slice(b, a, 26, 0, &mut start_index, 24);
        copy_slice(b, a, 27, 0, &mut start_index, 23);
        copy_slice(b, a, 28, 0, &mut start_index, 23);
        copy_slice(b, a, 29, 0, &mut start_index, 22);
        copy_slice(b, a, 30, 0, &mut start_index, 22);
        copy_slice(b, a, 31, 0, &mut start_index, 21);
        copy_slice(b, a, 32, 0, &mut start_index, 21);
        copy_slice(b, a, 33, 0, &mut start_index, 20);
        copy_slice(b, a, 34, 0, &mut start_index, 20);
        copy_slice(b, a, 35, 0, &mut start_index, 19);
        copy_slice(b, a, 36, 0, &mut start_index, 19);
        copy_slice(b, a, 37, 0, &mut start_index, 18);
        copy_slice(b, a, 38, 0, &mut start_index, 18);
        copy_slice(b, a, 39, 0, &mut start_index, 17);
        copy_slice(b, a, 40, 0, &mut start_index, 17);
        copy_slice(b, a, 41, 0, &mut start_index, 16);
        copy_slice(b, a, 42, 0, &mut start_index, 16);
        copy_slice(b, a, 43, 0, &mut start_index, 15);
        copy_slice(b, a, 44, 0, &mut start_index, 15);
        copy_slice(b, a, 45, 0, &mut start_index, 14);
        copy_slice(b, a, 46, 0, &mut start_index, 14);
        copy_slice(b, a, 47, 0, &mut start_index, 13);
        copy_slice(b, a, 48, 0, &mut start_index, 13);
        copy_slice(b, a, 49, 0, &mut start_index, 12);
        copy_slice(b, a, 50, 0, &mut start_index, 12);
        copy_slice(b, a, 51, 0, &mut start_index, 11);
        copy_slice(b, a, 52, 0, &mut start_index, 11);
        copy_slice(b, a, 53, 0, &mut start_index, 10);
        copy_slice(b, a, 54, 0, &mut start_index, 10);
        copy_slice(b, a, 55, 0, &mut start_index, 9);

        AnimeDataBuffer::from_vec(crate::AnimeType::GA402, buf)
    }

    fn to_gu604_packets(&self) -> Result<AnimeDataBuffer> {
        let mut buf = vec![0u8; AnimeType::GU604.data_length()];
        let mut start_index: usize = 0;

        fn copy_slice(
            buf: &mut [u8],
            anime: &AnimeDiagonal,
            x: usize,
            y: usize,
            start_index: &mut usize,
            len: usize,
        ) {
            buf[*start_index..*start_index + len].copy_from_slice(&anime.get_row(x, y, len));
            *start_index += len;
        }

        let b = &mut buf;
        let a = &self;
        copy_slice(b, a, 0, 4, &mut start_index, 38);
        copy_slice(b, a, 0, 3, &mut start_index, 39);
        copy_slice(b, a, 1, 3, &mut start_index, 38);
        copy_slice(b, a, 1, 2, &mut start_index, 39);
        copy_slice(b, a, 2, 2, &mut start_index, 38);
        copy_slice(b, a, 2, 1, &mut start_index, 39);
        copy_slice(b, a, 3, 1, &mut start_index, 38);
        copy_slice(b, a, 3, 0, &mut start_index, 39);
        copy_slice(b, a, 4, 0, &mut start_index, 39);
        copy_slice(b, a, 5, 0, &mut start_index, 39);
        copy_slice(b, a, 6, 0, &mut start_index, 38);
        copy_slice(b, a, 7, 0, &mut start_index, 38);
        copy_slice(b, a, 8, 0, &mut start_index, 37);
        copy_slice(b, a, 9, 0, &mut start_index, 37);
        copy_slice(b, a, 10, 0, &mut start_index, 36);
        copy_slice(b, a, 11, 0, &mut start_index, 36);
        copy_slice(b, a, 12, 0, &mut start_index, 35);
        copy_slice(b, a, 13, 0, &mut start_index, 35);
        copy_slice(b, a, 14, 0, &mut start_index, 34);
        copy_slice(b, a, 15, 0, &mut start_index, 34);
        copy_slice(b, a, 16, 0, &mut start_index, 33);
        copy_slice(b, a, 17, 0, &mut start_index, 33);
        copy_slice(b, a, 18, 0, &mut start_index, 32);
        copy_slice(b, a, 19, 0, &mut start_index, 32);
        copy_slice(b, a, 20, 0, &mut start_index, 31);
        copy_slice(b, a, 21, 0, &mut start_index, 31);
        copy_slice(b, a, 22, 0, &mut start_index, 30);
        copy_slice(b, a, 23, 0, &mut start_index, 30);
        copy_slice(b, a, 24, 0, &mut start_index, 29);
        copy_slice(b, a, 25, 0, &mut start_index, 29);
        copy_slice(b, a, 26, 0, &mut start_index, 28);
        copy_slice(b, a, 27, 0, &mut start_index, 28);
        copy_slice(b, a, 28, 0, &mut start_index, 27);
        copy_slice(b, a, 29, 0, &mut start_index, 27);
        copy_slice(b, a, 30, 0, &mut start_index, 26);
        copy_slice(b, a, 31, 0, &mut start_index, 26);
        copy_slice(b, a, 32, 0, &mut start_index, 25);
        copy_slice(b, a, 33, 0, &mut start_index, 25);
        copy_slice(b, a, 34, 0, &mut start_index, 24);
        copy_slice(b, a, 35, 0, &mut start_index, 24);
        copy_slice(b, a, 36, 0, &mut start_index, 23);
        copy_slice(b, a, 37, 0, &mut start_index, 23);
        copy_slice(b, a, 38, 0, &mut start_index, 22);
        copy_slice(b, a, 39, 0, &mut start_index, 22);
        copy_slice(b, a, 40, 0, &mut start_index, 21);
        copy_slice(b, a, 41, 0, &mut start_index, 21);
        copy_slice(b, a, 42, 0, &mut start_index, 20);
        copy_slice(b, a, 43, 0, &mut start_index, 20);
        copy_slice(b, a, 44, 0, &mut start_index, 19);
        copy_slice(b, a, 45, 0, &mut start_index, 19);
        copy_slice(b, a, 46, 0, &mut start_index, 18);
        copy_slice(b, a, 47, 0, &mut start_index, 18);
        copy_slice(b, a, 48, 0, &mut start_index, 17);
        copy_slice(b, a, 49, 0, &mut start_index, 17);
        copy_slice(b, a, 50, 0, &mut start_index, 16);
        copy_slice(b, a, 51, 0, &mut start_index, 16);
        copy_slice(b, a, 52, 0, &mut start_index, 15);
        copy_slice(b, a, 53, 0, &mut start_index, 15);
        copy_slice(b, a, 54, 0, &mut start_index, 14);
        copy_slice(b, a, 55, 0, &mut start_index, 14);
        copy_slice(b, a, 56, 0, &mut start_index, 13);
        copy_slice(b, a, 57, 0, &mut start_index, 13);
        copy_slice(b, a, 58, 0, &mut start_index, 12);

        AnimeDataBuffer::from_vec(crate::AnimeType::GA402, buf)
    }

    /// Pack a STRIX-class image into the LED-ordered byte buffer.
    ///
    /// G635L and G835L share the same physical LED layout (inverted
    /// staggered matrix: rows 0-27 grow from 1→14 LEDs in pairs, rows
    /// 28-67 stay at 15 LEDs). This helper holds that ordering logic;
    /// per-device behaviors (buffer size, USB packet chunking) live in
    /// the callers.
    ///
    /// Per G Helper's `AnimeMatrixDevice` (STRIX class):
    ///   https://github.com/seerge/g-helper/blob/main/app/AnimeMatrix/AnimeMatrixDevice.cs
    ///
    /// Diagonal PNG layout:
    /// - Image height = 34 (row pairs)
    /// - Image width  = 68 (half-step X grid)
    /// - Even/odd rows are interleaved in X (staggered by 0.5 LED = 1 px)
    fn pack_strix_diagonal_into(buf: &mut [u8], image: &[Vec<u8>]) {
        use log::debug;

        let mut buf_idx = 0usize;

        debug!(
            "STRIX packing: image dimensions {}x{}, buffer size {}",
            image.first().map(|r| r.len()).unwrap_or(0),
            image.len(),
            buf.len()
        );

        // Helper: get row length for STRIX class (G635L/G835L)
        fn row_length(row: usize) -> usize {
            if row < 28 { row / 2 + 1 } else { 15 }
        }

        // Helper: starting X (in LED units) for the row
        fn first_x(row: usize) -> usize {
            if row < 28 { 0 } else { (row - 28) / 2 }
        }

        // Process all 68 rows
        for row in 0..68 {
            let len = row_length(row);
            let img_y = row / 2;
            let base_x = first_x(row);
            let stagger = row % 2;

            for i in 0..len {
                // Half-step X grid: even rows on even pixels, odd rows on odd pixels.
                let img_x = (base_x + i) * 2 + stagger;

                // Read from image, clamp to bounds
                let val = if img_y < image.len() && img_x < image[img_y].len() {
                    image[img_y][img_x]
                } else {
                    0
                };

                // Log first LED of each row for debugging
                if i == 0 {
                    debug!("Row {row}: len={len}, first LED at img[{img_y}][{img_x}] = {val}");
                }

                if buf_idx < buf.len() {
                    buf[buf_idx] = val;
                }
                buf_idx += 1;
            }
        }

        debug!("STRIX packing complete: {buf_idx} bytes written");
    }

    /// STRIX-class diagonal packing (G635L, G835L). Caller passes the
    /// concrete `AnimeType` so the resulting `AnimeDataBuffer` carries
    /// the right device identity downstream.
    fn to_strix_packets(&self, anime_type: AnimeType) -> Result<AnimeDataBuffer> {
        let mut buf = vec![0u8; anime_type.data_length()];
        Self::pack_strix_diagonal_into(&mut buf, &self.1);
        AnimeDataBuffer::from_vec(anime_type, buf)
    }
}
