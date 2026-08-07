use std::convert::TryFrom;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use glam::Vec2;
use log::error;
use serde::{Deserialize, Serialize};

use crate::error::{AnimeError, Result};
use crate::{AnimeDataBuffer, AnimeDiagonal, AnimeImage, AnimeType, Pixel};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimeFrame {
    /// Precomputed data for the frame. This can be transferred directly to the
    /// the `asusd` daemon over dbus or converted to USB packet with
    /// `AnimePacketType::from(buffer)`
    data: AnimeDataBuffer,
    delay: Duration,
}

impl AnimeFrame {
    /// Get the inner data buffer of the gif frame
    #[inline]
    pub fn frame(&self) -> &AnimeDataBuffer {
        &self.data
    }

    /// Get the `Duration` of the delay for this frame
    #[inline]
    pub fn delay(&self) -> Duration {
        self.delay
    }
}

/// Defines the time or animation cycle count to use for a gif
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum AnimTime {
    /// Time in milliseconds for animation to run
    Time(Duration),
    /// How many full animation loops to run or how many seconds if image is
    /// static
    Count(u32),
    /// Run for infinite time
    Infinite,
    /// Fade in, play for, fade out
    Fade(Fade),
}

impl Default for AnimTime {
    #[inline]
    fn default() -> Self {
        Self::Infinite
    }
}

/// Fancy brightness control: fade in/out, show at brightness for n time
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct Fade {
    fade_in: Duration,
    show_for: Option<Duration>,
    fade_out: Duration,
}

impl Fade {
    pub fn new(fade_in: Duration, show_for: Option<Duration>, fade_out: Duration) -> Self {
        Self {
            fade_in,
            show_for,
            fade_out,
        }
    }

    pub fn fade_in(&self) -> Duration {
        self.fade_in
    }

    pub fn show_for(&self) -> Option<Duration> {
        self.show_for
    }

    pub fn fade_out(&self) -> Duration {
        self.fade_out
    }

    pub fn total_fade_time(&self) -> Duration {
        self.fade_in + self.fade_out
    }
}

/// A gif animation. This is a collection of frames from the gif, and a duration
/// that the animation should be shown for.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimeGif(Vec<AnimeFrame>, AnimTime);

impl AnimeGif {
    /// Create an animation using the 74x36 ASUS gif format
    #[inline]
    pub fn from_diagonal_gif(
        file_name: &Path,
        duration: AnimTime,
        brightness: f32,
        anime_type: AnimeType,
    ) -> Result<Self> {
        let mut matrix = AnimeDiagonal::new(anime_type);

        let mut decoder = gif::DecodeOptions::new();
        // Configure the decoder such that it will expand the image to RGBA.
        decoder.set_color_output(gif::ColorOutput::RGBA);
        // Read the file header
        let file = File::open(file_name).map_err(|e| {
            error!("Could not open {file_name:?}: {e:?}");
            e
        })?;
        let mut decoder = decoder.read_info(file)?;

        let mut frames = Vec::default();
        while let Some(frame) = decoder.read_next_frame()? {
            let wait = frame.delay * 10;
            // if matches!(frame.dispose, gif::DisposalMethod::Background) {
            //     frames = Vec::new();
            // }
            for (y, row) in frame.buffer.chunks(frame.width as usize * 4).enumerate() {
                for (x, px) in row.chunks(4).enumerate() {
                    if px[3] != 255 {
                        // should be t but not in some gifs? What, ASUS, what?
                        continue;
                    }
                    let tmp = matrix.get_mut();
                    let y = y + frame.top as usize;
                    if y >= tmp.len() {
                        return Err(AnimeError::PixelGifHeight(tmp.len()));
                    }
                    let x = x + frame.left as usize;
                    if x >= tmp[y].len() {
                        return Err(AnimeError::PixelGifWidth(tmp[y].len()));
                    }

                    matrix.get_mut()[y][x] = (px[0] as f32 * brightness) as u8;
                }
            }

            frames.push(AnimeFrame {
                data: matrix.into_data_buffer(anime_type)?,
                delay: Duration::from_millis(wait as u64),
            });
        }
        Ok(Self(frames, duration))
    }

    /// Create an animation using the 74x36 ASUS gif format from a png
    #[inline]
    pub fn from_diagonal_png(
        file_name: &Path,
        anime_type: AnimeType,
        duration: AnimTime,
        brightness: f32,
    ) -> Result<Self> {
        let image = AnimeDiagonal::from_png(file_name, brightness, anime_type)?;

        let mut total = Duration::from_millis(1000);
        if let AnimTime::Fade(fade) = duration {
            total = fade.total_fade_time();
            if let Some(middle) = fade.show_for {
                total += middle;
            }
        }
        // Make frame delay 30ms, and find frame count
        let frame_count = total.as_millis() / 30;

        let single = AnimeFrame {
            data: image.into_data_buffer(anime_type)?,
            delay: Duration::from_millis(30),
        };
        let frames = vec![single; frame_count as usize];

        Ok(Self(frames, duration))
    }

    /// Create an animation using a gif of any size. This method must precompute
    /// the result.
    #[inline]
    pub fn from_gif(
        file_name: &Path,
        scale: f32,
        angle: f32,
        translation: Vec2,
        duration: AnimTime,
        brightness: f32,
        anime_type: AnimeType,
    ) -> Result<Self> {
        let mut frames = Vec::new();
        let mut decoder = gif::DecodeOptions::new();
        // Configure the decoder such that it will expand the image to RGBA.
        decoder.set_color_output(gif::ColorOutput::RGBA);
        // Read the file header
        let file = File::open(file_name).map_err(|e| {
            error!("Could not open {file_name:?}: {e:?}");
            e
        })?;
        let mut decoder = decoder.read_info(file)?;

        let height = decoder.height();
        let width = decoder.width();
        let pixels: Vec<Pixel> =
            vec![Pixel::default(); (decoder.width() as u32 * decoder.height() as u32) as usize];
        let mut image = AnimeImage::new(
            Vec2::new(scale, scale),
            angle,
            translation,
            brightness,
            pixels,
            decoder.width() as u32,
            anime_type,
        )?;

        while let Some(frame) = decoder.read_next_frame()? {
            let wait = frame.delay * 10;
            if matches!(frame.dispose, gif::DisposalMethod::Background) {
                let pixels: Vec<Pixel> =
                    vec![Pixel::default(); (width as u32 * height as u32) as usize];
                image = AnimeImage::new(
                    Vec2::new(scale, scale),
                    angle,
                    translation,
                    brightness,
                    pixels,
                    width as u32,
                    anime_type,
                )?;
            }
            for (y, row) in frame.buffer.chunks(frame.width as usize * 4).enumerate() {
                for (x, px) in row.chunks(4).enumerate() {
                    if px[3] != 255 {
                        // should be t but not in some gifs? What, ASUS, what?
                        continue;
                    }
                    let pos =
                        (x + frame.left as usize) + ((y + frame.top as usize) * width as usize);
                    image.get_mut()[pos] = Pixel {
                        color: ((px[0] as u32 + px[1] as u32 + px[2] as u32) / 3),
                        alpha: 1.0,
                    };
                }
            }
            image.update();

            frames.push(AnimeFrame {
                data: <AnimeDataBuffer>::try_from(&image)?,
                delay: Duration::from_millis(wait as u64),
            });
        }
        Ok(Self(frames, duration))
    }

    /// Make a static gif out of a greyscale png. If no duration is specified
    /// then the default will be 1 second long. If `AnimTime::Cycles` is
    /// specified for `duration` then this can be considered how many
    /// seconds the image will show for.
    #[inline]
    pub fn from_png(
        file_name: &Path,
        scale: f32,
        angle: f32,
        translation: Vec2,
        duration: AnimTime,
        brightness: f32,
        anime_type: AnimeType,
    ) -> Result<Self> {
        let image =
            AnimeImage::from_png(file_name, scale, angle, translation, brightness, anime_type)?;

        let mut total = Duration::from_millis(1000);
        if let AnimTime::Fade(fade) = duration {
            total = fade.total_fade_time();
            if let Some(middle) = fade.show_for {
                total += middle;
            }
        }
        // Make frame delay 30ms, and find frame count
        let frame_count = total.as_millis() / 30;

        let single = AnimeFrame {
            data: <AnimeDataBuffer>::try_from(&image)?,
            delay: Duration::from_millis(30),
        };
        let frames = vec![single; frame_count as usize];

        Ok(Self(frames, duration))
    }

    /// Get a slice of the frames this gif has
    #[inline]
    pub fn frames(&self) -> &[AnimeFrame] {
        &self.0
    }

    /// Get the time/count for this gif
    #[inline]
    pub fn duration(&self) -> AnimTime {
        self.1
    }

    /// Get the frame count
    pub fn frame_count(&self) -> usize {
        self.0.len()
    }

    /// Get total gif time for one run
    pub fn total_frame_time(&self) -> Duration {
        let mut time = 0;
        self.0.iter().for_each(|f| time += f.delay.as_millis());
        Duration::from_millis(time as u64)
    }
}
