use std::convert::TryFrom;
use std::path::PathBuf;
use std::time::Duration;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::{AnimTime, AnimeDataBuffer, AnimeDiagonal, AnimeGif, AnimeImage, AnimeType};

/// All the possible `AniMe` actions that can be used. This enum is intended to
/// be a helper for loading up `ActionData`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ActionLoader {
    /// Full gif sequence. Immutable.
    AsusAnimation {
        file: PathBuf,
        time: AnimTime,
        brightness: f32,
    },
    /// Image designed to be pixel perfect using the slanted template
    AsusImage {
        file: PathBuf,
        time: AnimTime,
        brightness: f32,
    },
    /// Animated gif. If the file is a png a static gif is created using the
    /// `time` properties
    ImageAnimation {
        file: PathBuf,
        scale: f32,
        angle: f32,
        translation: Vec2,
        time: AnimTime,
        brightness: f32,
    },
    Image {
        file: PathBuf,
        scale: f32,
        angle: f32,
        translation: Vec2,
        time: AnimTime,
        brightness: f32,
    },
    /// A pause to be used between sequences
    Pause(Duration),
}

/// All the possible `AniMe` actions that can be used. The enum is intended to
/// be used in a array allowing the user to cycle through a series of actions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ActionData {
    /// Full gif sequence. Immutable.
    Animation(AnimeGif),
    /// Basic image, can have properties changed and image updated via those
    /// properties
    Image(Box<AnimeDataBuffer>),
    /// A pause to be used between sequences
    Pause(Duration),
    /// Placeholder
    AudioEq,
    /// Placeholder
    SystemInfo,
    /// Placeholder
    TimeDate,
    /// Placeholder
    Matrix,
}

impl ActionData {
    pub fn from_anime_action(anime_type: AnimeType, action: &ActionLoader) -> Result<ActionData> {
        let a = match action {
            ActionLoader::AsusAnimation {
                file,
                time,
                brightness,
            } => ActionData::Animation(AnimeGif::from_diagonal_gif(
                file, *time, *brightness, anime_type,
            )?),
            ActionLoader::AsusImage {
                file,
                time,
                brightness,
            } => match time {
                AnimTime::Infinite => {
                    let image = AnimeDiagonal::from_png(file, None, *brightness, anime_type)?;
                    let data = image.into_data_buffer(anime_type)?;
                    ActionData::Image(Box::new(data))
                }
                _ => ActionData::Animation(AnimeGif::from_diagonal_png(
                    file, anime_type, *time, *brightness,
                )?),
            },
            ActionLoader::ImageAnimation {
                file,
                scale,
                angle,
                translation,
                time,
                brightness,
            } => {
                if let Some(ext) = file.extension()
                    && ext.to_string_lossy().to_lowercase() == "png"
                {
                    return Ok(ActionData::Animation(AnimeGif::from_png(
                        file, *scale, *angle, *translation, *time, *brightness, anime_type,
                    )?));
                }
                ActionData::Animation(AnimeGif::from_gif(
                    file, *scale, *angle, *translation, *time, *brightness, anime_type,
                )?)
            }
            ActionLoader::Image {
                file,
                scale,
                angle,
                translation,
                brightness,
                time,
            } => {
                match time {
                    AnimTime::Infinite => {
                        // If no time then create a plain static image
                        let image = AnimeImage::from_png(
                            file, *scale, *angle, *translation, *brightness, anime_type,
                        )?;
                        let data = <AnimeDataBuffer>::try_from(&image)?;
                        ActionData::Image(Box::new(data))
                    }
                    _ => ActionData::Animation(AnimeGif::from_png(
                        file, *scale, *angle, *translation, *time, *brightness, anime_type,
                    )?),
                }
            }
            ActionLoader::Pause(duration) => ActionData::Pause(*duration),
        };
        Ok(a)
    }
}

/// An optimised precomputed set of actions that the user can cycle through
#[derive(Debug, Deserialize, Serialize)]
pub struct Sequences(Vec<ActionData>, AnimeType);

impl Sequences {
    #[inline]
    pub fn new(anime_type: AnimeType) -> Self {
        Self(Vec::new(), anime_type)
    }

    /// Use a base `AnimeAction` to generate the precomputed data and insert in
    /// to the run buffer
    #[inline]
    pub fn insert(&mut self, index: usize, action: &ActionLoader) -> Result<()> {
        self.0
            .insert(index, ActionData::from_anime_action(self.1, action)?);
        Ok(())
    }

    /// Remove an item at this position from the run buffer. If the `index`
    /// supplied is not in range then `None` is returned, otherwise the
    /// `ActionData` at that location is yeeted and returned.
    #[inline]
    pub fn remove_item(&mut self, index: usize) -> Option<ActionData> {
        if index < self.0.len() {
            return Some(self.0.remove(index));
        }
        None
    }

    pub fn iter(&self) -> ActionIterator<'_> {
        ActionIterator {
            actions: self,
            next_idx: 0,
        }
    }
}

/// Iteractor helper for iterating over all the actions in `Sequences`
pub struct ActionIterator<'a> {
    actions: &'a Sequences,
    next_idx: usize,
}

impl<'a> Iterator for ActionIterator<'a> {
    type Item = &'a ActionData;

    #[inline]
    fn next(&mut self) -> Option<&'a ActionData> {
        if self.next_idx == self.actions.0.len() {
            self.next_idx = 0;
            return None;
        }

        let current = self.next_idx;
        self.next_idx += 1;

        Some(&self.actions.0[current])
    }
}
