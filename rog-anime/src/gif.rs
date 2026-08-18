use std::convert::TryFrom;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use glam::Vec2;
use image::ImageDecoder;
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

impl AnimTime {
    /// Calculate the frame count for a static image with 30ms frame delay
    #[inline]
    pub fn static_frame_count(&self) -> usize {
        let mut total = Duration::from_millis(1000);
        if let AnimTime::Fade(fade) = self {
            total = fade.total_fade_time();
            if let Some(middle) = fade.show_for {
                total += middle;
            }
        }
        (total.as_millis() / 30).max(1) as usize
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

fn decode_gif(file_name: &Path) -> Result<(image::Frames<'static>, u32, u32)> {
    let file = File::open(file_name).inspect_err(|e| {
        error!("Could not open {file_name:?}: {e:?}");
    })?;
    let mut decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file))?;
    decoder.set_limits(image::Limits::default())?;
    let (width, height) = decoder.dimensions();
    Ok((image::AnimationDecoder::into_frames(decoder), width, height))
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

        let mut frames = Vec::default();
        let (frames_iter, _, _) = decode_gif(file_name)?;
        for frame in frames_iter {
            let frame = frame?;
            let wait: Duration = frame.delay().into();
            let left = frame.left() as usize;
            let top = frame.top() as usize;
            let buffer = frame.buffer();

            for (x, y, px) in buffer.enumerate_pixels() {
                if px.0[3] != 255 {
                    // should be t but not in some gifs? What, ASUS, what?
                    continue;
                }
                let tmp = matrix.get_mut();
                let y = y as usize + top;
                let x = x as usize + left;
                if y >= tmp.len() {
                    return Err(AnimeError::PixelGifHeight(tmp.len()));
                }
                if x >= tmp[y].len() {
                    return Err(AnimeError::PixelGifWidth(tmp[y].len()));
                }

                let v = Pixel::from(px).color as f32;
                tmp[y][x] = (v * brightness) as u8;
            }

            frames.push(AnimeFrame {
                data: matrix.into_data_buffer(anime_type)?,
                delay: wait,
            });
        }
        if frames.is_empty() {
            return Err(AnimeError::NoFrames);
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
        let frame_count = duration.static_frame_count();

        let single = AnimeFrame {
            data: image.into_data_buffer(anime_type)?,
            delay: Duration::from_millis(30),
        };
        let frames = vec![single; frame_count];

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
        let (frames_iter, width_u32, height_u32) = decode_gif(file_name)?;
        let width = width_u32 as usize;
        let height = height_u32 as usize;

        let pixels: Vec<Pixel> = vec![Pixel::default(); width * height];
        let mut anime_image = AnimeImage::new(
            Vec2::new(scale, scale),
            angle,
            translation,
            brightness,
            pixels,
            width_u32,
            anime_type,
        )?;

        let mut frames = Vec::new();
        for frame in frames_iter {
            let frame = frame?;
            let wait: Duration = frame.delay().into();
            let left = frame.left() as usize;
            let top = frame.top() as usize;
            let buffer = frame.buffer();

            for (x, y, px) in buffer.enumerate_pixels() {
                if px.0[3] != 255 {
                    // should be t but not in some gifs? What, ASUS, what?
                    continue;
                }
                let px_x = x as usize + left;
                let px_y = y as usize + top;
                if px_x >= width || px_y >= height {
                    continue;
                }
                let pos = px_x + px_y * width;
                anime_image.get_mut()[pos] = Pixel::from(px);
            }
            anime_image.update();

            frames.push(AnimeFrame {
                data: <AnimeDataBuffer>::try_from(&anime_image)?,
                delay: wait,
            });
        }
        if frames.is_empty() {
            return Err(AnimeError::NoFrames);
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
        let frame_count = duration.static_frame_count();

        let single = AnimeFrame {
            data: <AnimeDataBuffer>::try_from(&image)?,
            delay: Duration::from_millis(30),
        };
        let frames = vec![single; frame_count];

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
        self.0.iter().map(|f| f.delay).sum()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_from_gif_custom() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("data/anime/custom/sonic-run.gif");

        let gif = AnimeGif::from_gif(
            &path,
            1.0,
            0.0,
            Vec2::default(),
            AnimTime::Infinite,
            1.0,
            AnimeType::GA402,
        )
        .expect("Failed to decode sonic-run.gif");

        assert!(gif.frame_count() > 0);
        assert!(gif.total_frame_time() > Duration::ZERO);
    }

    #[test]
    fn test_from_diagonal_gif_ga401() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/data/ga401-diagonal.gif");

        let gif = AnimeGif::from_diagonal_gif(&path, AnimTime::Count(1), 1.0, AnimeType::GA401)
            .expect("Failed to decode ga401-diagonal.gif");

        assert_eq!(gif.frame_count(), 1);
    }

    #[test]
    fn test_from_diagonal_gif_ga402() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/data/ga402-diagonal.gif");

        let gif = AnimeGif::from_diagonal_gif(&path, AnimTime::Count(1), 1.0, AnimeType::GA402)
            .expect("Failed to decode ga402-diagonal.gif");

        assert_eq!(gif.frame_count(), 1);
    }

    #[test]
    fn test_from_diagonal_gif_g835l() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/data/g835l-diagonal.gif");

        let gif = AnimeGif::from_diagonal_gif(&path, AnimTime::Count(1), 1.0, AnimeType::G835L)
            .expect("Failed to decode g835l-diagonal.gif");

        // 48 is the expected image-frame count for the g835l-diagonal.gif fixture
        assert_eq!(gif.frame_count(), 48);
    }
}
