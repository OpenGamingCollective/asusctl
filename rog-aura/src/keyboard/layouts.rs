//! A series of pre-defined layouts. These were mostly used to generate an
//! editable config.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::slice::Iter;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::aura_detection::LedSupportData;
use crate::error::Error;
use crate::keyboard::{AdvancedAuraType, LedCode};
use crate::{AuraModeNum, AuraZone};

/// The `key_type` plays a role in effects (eventually). You could for example
/// add a `ShapeType::Spacing` to pad out an effect, such as a laserbeam across
/// a row so that it doesn't appear to *jump* across a gap
///
/// w=1.0, h=1.0 should be considered the size of a typical key like 'A'
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum KeyShape {
    Led {
        width: f32,
        height: f32,
        pad_left: f32,
        pad_right: f32,
        pad_top: f32,
        pad_bottom: f32,
    },
    Blank {
        width: f32,
        height: f32,
    },
}

impl KeyShape {
    pub fn new_led(
        width: f32,
        height: f32,
        pad_left: f32,
        pad_right: f32,
        pad_top: f32,
        pad_bottom: f32,
    ) -> Self {
        Self::Led {
            width,
            height,
            pad_left,
            pad_right,
            pad_top,
            pad_bottom,
        }
    }

    pub fn new_blank(width: f32, height: f32) -> Self {
        Self::Blank { width, height }
    }

    /// Scale the shape up/down. Intended for use in UI on a clone
    pub fn scale(&mut self, scale: f32) {
        match self {
            KeyShape::Led {
                width,
                height,
                pad_left,
                pad_right,
                pad_top,
                pad_bottom,
            } => {
                *width *= scale;
                *height *= scale;
                *pad_left *= scale;
                *pad_right *= scale;
                *pad_top *= scale;
                *pad_bottom *= scale;
            }
            KeyShape::Blank { width, height } => {
                *width *= scale;
                *height *= scale;
            }
        }
    }
}

/// The first `Key` will determine the row height.
///
/// Every row is considered to start a x=0, with the first row being y=0,
/// and following rows starting after the previous `row_y + pad_top` and
/// `row_x + pad_left`
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyRow {
    pad_left: f32,
    pad_top: f32,
    /// The `Key` is what provides an RGB index location in the final USB
    /// packets
    row: Vec<(LedCode, String)>,
    /// The final data structure merged key_shapes and rows
    #[serde(skip)]
    built_row: Vec<(LedCode, KeyShape)>,
}

impl KeyRow {
    pub fn new(pad_left: f32, pad_top: f32, row: Vec<(LedCode, String)>) -> Self {
        Self {
            pad_left,
            pad_top,
            row,
            built_row: Default::default(),
        }
    }

    pub fn row(&self) -> Iter<'_, (LedCode, KeyShape)> {
        self.built_row.iter()
    }

    pub fn row_ref(&self) -> &[(LedCode, KeyShape)] {
        &self.built_row
    }

    /// Find and return the heightest height of this row
    pub fn height(&self) -> f32 {
        if self.built_row.is_empty() {
            return 0.0;
        }
        let mut h = 0.0;
        for k in &self.built_row {
            let height = match &k.1 {
                KeyShape::Led {
                    height,
                    pad_top,
                    pad_bottom,
                    ..
                } => height + pad_top + pad_bottom,
                KeyShape::Blank { height, .. } => *height,
            };

            if h < height {
                h = height;
            }
        }
        h
    }

    /// Return the total row width
    pub fn width(&self) -> f32 {
        if self.built_row.is_empty() {
            return 0.0;
        }
        let mut w = 0.0;
        for k in &self.built_row {
            match &k.1 {
                KeyShape::Led {
                    width,
                    pad_left,
                    pad_right,
                    ..
                } => w += width + pad_left + pad_right,
                KeyShape::Blank { width, .. } => w += width,
            }
        }
        w
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyLayout {
    /// Localization of this keyboard layout
    locale: String,
    /// The shapes of keys used
    key_shapes: HashMap<String, KeyShape>,
    /// The rows of keys of this layout
    key_rows: Vec<KeyRow>,
    /// Should be copied from the `LaptopLedData` as laptops may have the same
    /// layout, but different EC features
    #[serde(skip)]
    basic_modes: Vec<AuraModeNum>,
    /// Should be copied from the `LaptopLedData` as laptops may have the same
    /// layout, but different EC features
    #[serde(skip)]
    basic_zones: Vec<AuraZone>,
    /// Paired with the key selection in UI. Determines if individual keys are
    /// selectable, zones, or single zone.
    ///
    /// Should be copied from the `LaptopLedData` as laptops may have the same
    /// layout, but different EC features.
    #[serde(skip)]
    advanced_type: AdvancedAuraType,
}

impl KeyLayout {
    fn from_file(path: &Path) -> Result<Self, Error> {
        let buf: String = std::fs::read_to_string(path)
            .map_err(|e| Error::IoPath(path.to_string_lossy().to_string(), e))?;
        if buf.is_empty() {
            Err(Error::IoPath(
                path.to_string_lossy().to_string(),
                std::io::ErrorKind::InvalidData.into(),
            ))
        } else {
            let mut data = ron::from_str::<Self>(&buf)?;

            let mut unused = HashSet::new();
            for k in data.key_shapes.keys() {
                unused.insert(k);
            }

            let rows = &mut data.key_rows;
            for row in rows {
                for k in &mut row.row {
                    if let Some(shape) = data.key_shapes.get(&k.1) {
                        row.built_row.push((k.0, shape.clone()));
                        unused.remove(&k.1);
                    } else {
                        warn!("Key {:?} was missing matching shape {}", k.0, k.1);
                    }
                }
            }

            if !unused.is_empty() {
                warn!("The layout {path:?} had unused shapes {unused:?}",);
            }

            Ok(data)
        }
    }

    pub fn rows(&self) -> Iter<'_, KeyRow> {
        self.key_rows.iter()
    }

    pub fn rows_ref(&self) -> &[KeyRow] {
        &self.key_rows
    }

    pub fn basic_modes(&self) -> &[AuraModeNum] {
        &self.basic_modes
    }

    pub fn basic_zones(&self) -> &[AuraZone] {
        &self.basic_zones
    }

    pub fn advanced_type(&self) -> &AdvancedAuraType {
        &self.advanced_type
    }

    /// Find the total heighht of the keyboard, not including lightbar rows
    pub fn keyboard_height(&self) -> f32 {
        let mut height = 0.0;
        for r in &self.key_rows {
            if let Some(key) = r.row.first()
                && !key.0.is_lightbar_zone()
            {
                height += r.height();
            }
        }
        height
    }

    pub fn max_height(&self) -> f32 {
        let mut height = 0.0;
        for r in &self.key_rows {
            height += r.height();
        }
        height
    }

    pub fn max_width(&self) -> f32 {
        let mut width = 0.0;
        for r in &self.key_rows {
            let tmp = r.width();
            if width < tmp {
                width = tmp;
            }
        }
        width
    }

    /// Find a layout matching the name in `LaptopLedData` in the provided dir
    pub fn find_layout(led_data: LedSupportData, mut data_path: PathBuf) -> Result<Self, Error> {
        // TODO: locales
        let layout_name = if led_data.layout_name.is_empty() {
            "ga401q".to_owned() // Need some sort of default here due to ROGCC
        // expecting it
        } else {
            led_data.layout_name
        };
        let layout_file = format!("{layout_name}_US.ron");
        data_path.push("layouts");
        data_path.push(layout_file);
        let path = data_path.as_path();

        let mut tmp = KeyLayout::from_file(path)?;
        tmp.basic_modes = led_data.basic_modes;
        tmp.basic_zones = led_data.basic_zones;
        tmp.advanced_type = led_data.advanced_type;

        Ok(tmp)
    }

    pub fn layout_files(mut data_path: PathBuf) -> Result<Vec<PathBuf>, Error> {
        data_path.push("layouts");
        let path = data_path.as_path();
        let mut files = Vec::new();
        std::fs::read_dir(path)
            .map_err(|e| {
                println!("{path:?}, {e}");
                e
            })
            .unwrap()
            .for_each(|p| {
                if let Ok(p) = p {
                    files.push(p.path());
                }
            });

        Ok(files)
    }
}

impl KeyLayout {
    pub fn default_layout() -> Self {
        Self {
            locale: "US".to_owned(),
            basic_modes: vec![
                AuraModeNum::Static,
                AuraModeNum::Breathe,
                AuraModeNum::Pulse,
            ],
            basic_zones: vec![AuraZone::None],
            advanced_type: AdvancedAuraType::None,
            key_shapes: HashMap::from([(
                "regular".to_owned(),
                KeyShape::new_led(1.0, 1.0, 0.1, 0.1, 0.1, 0.1),
            )]),
            key_rows: vec![
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::Esc, "regular".to_owned()),
                        (LedCode::F1, "regular".to_owned()),
                        (LedCode::F2, "regular".to_owned()),
                        (LedCode::F3, "regular".to_owned()),
                        (LedCode::F4, "regular".to_owned()),
                        // not sure which key to put here
                        (LedCode::F5, "regular".to_owned()),
                        (LedCode::F6, "regular".to_owned()),
                        (LedCode::F7, "regular".to_owned()),
                        (LedCode::F8, "regular".to_owned()),
                        (LedCode::F9, "regular".to_owned()),
                        (LedCode::F10, "regular".to_owned()),
                        (LedCode::F11, "regular".to_owned()),
                        (LedCode::F12, "regular".to_owned()),
                    ],
                ),
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::Tilde, "regular".to_owned()),
                        (LedCode::N1, "regular".to_owned()),
                        (LedCode::N2, "regular".to_owned()),
                        (LedCode::N3, "regular".to_owned()),
                        (LedCode::N4, "regular".to_owned()),
                        (LedCode::N5, "regular".to_owned()),
                        (LedCode::N6, "regular".to_owned()),
                        (LedCode::N7, "regular".to_owned()),
                        (LedCode::N8, "regular".to_owned()),
                        (LedCode::N9, "regular".to_owned()),
                        (LedCode::N0, "regular".to_owned()),
                        (LedCode::Hyphen, "regular".to_owned()),
                        (LedCode::Equals, "regular".to_owned()),
                        (LedCode::Backspace, "regular".to_owned()),
                    ],
                ),
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::Tab, "regular".to_owned()),
                        (LedCode::Q, "regular".to_owned()),
                        (LedCode::W, "regular".to_owned()),
                        (LedCode::E, "regular".to_owned()),
                        (LedCode::R, "regular".to_owned()),
                        (LedCode::T, "regular".to_owned()),
                        (LedCode::Y, "regular".to_owned()),
                        (LedCode::U, "regular".to_owned()),
                        (LedCode::I, "regular".to_owned()),
                        (LedCode::O, "regular".to_owned()),
                        (LedCode::P, "regular".to_owned()),
                        (LedCode::LBracket, "regular".to_owned()),
                        (LedCode::RBracket, "regular".to_owned()),
                        (LedCode::BackSlash, "regular".to_owned()),
                    ],
                ),
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::Caps, "regular".to_owned()),
                        (LedCode::A, "regular".to_owned()),
                        (LedCode::S, "regular".to_owned()),
                        (LedCode::D, "regular".to_owned()),
                        (LedCode::F, "regular".to_owned()),
                        (LedCode::G, "regular".to_owned()),
                        (LedCode::H, "regular".to_owned()),
                        (LedCode::J, "regular".to_owned()),
                        (LedCode::K, "regular".to_owned()),
                        (LedCode::L, "regular".to_owned()),
                        (LedCode::SemiColon, "regular".to_owned()),
                        (LedCode::Quote, "regular".to_owned()),
                        (LedCode::Return, "regular".to_owned()),
                    ],
                ),
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::LShift, "regular".to_owned()),
                        (LedCode::Z, "regular".to_owned()),
                        (LedCode::X, "regular".to_owned()),
                        (LedCode::C, "regular".to_owned()),
                        (LedCode::V, "regular".to_owned()),
                        (LedCode::B, "regular".to_owned()),
                        (LedCode::N, "regular".to_owned()),
                        (LedCode::M, "regular".to_owned()),
                        (LedCode::Comma, "regular".to_owned()),
                        (LedCode::Period, "regular".to_owned()),
                        (LedCode::FwdSlash, "regular".to_owned()),
                        (LedCode::Rshift, "regular".to_owned()),
                    ],
                ),
                KeyRow::new(
                    0.1,
                    0.1,
                    vec![
                        (LedCode::LCtrl, "regular".to_owned()),
                        (LedCode::LFn, "regular".to_owned()),
                        (LedCode::Meta, "regular".to_owned()),
                        (LedCode::LAlt, "regular".to_owned()),
                        (LedCode::Spacebar, "regular".to_owned()),
                        (LedCode::RAlt, "regular".to_owned()),
                        (LedCode::PrtSc, "regular".to_owned()),
                        (LedCode::RCtrl, "regular".to_owned()),
                    ],
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs::{self, OpenOptions};
    use std::io::Read;
    use std::path::PathBuf;

    use crate::aura_detection::LedSupportFile;
    use crate::keyboard::KeyLayout;

    #[test]
    fn check_parse_all() {
        const DATA_DIR: &str = env!("CARGO_MANIFEST_DIR");
        let mut data_path = PathBuf::from(DATA_DIR);

        data_path.push("data");
        data_path.push("layouts");
        let path = data_path.as_path();
        for p in fs::read_dir(path)
            .map_err(|e| {
                println!("{path:?}, {e}");
                e
            })
            .unwrap()
        {
            let mut buf = std::fs::read_to_string(p.unwrap().path()).unwrap();

            let data: KeyLayout = ron::from_str(&buf).unwrap();

            let mut unused = HashSet::new();
            for k in data.key_shapes.keys() {
                unused.insert(k);
            }

            let rows = &data.key_rows;
            for row in rows {
                for k in &row.row {
                    if data.key_shapes.contains_key(&k.1) {
                        unused.remove(&k.1);
                    } else {
                        panic!("Key {:?} was missing matching shape {}", k.0, k.1);
                    }
                }
            }

            assert!(
                unused.is_empty(),
                "The layout {path:?} had unused shapes {unused:?}",
            );
            buf.clear();
        }

        // println!(
        //     "RON: {}",
        //     ron::ser::to_string_pretty(&tmp,
        // PrettyConfig::new().depth_limit(4)).unwrap() );

        // let mut data = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // data.push("data/aura-support2.json");

        // let mut file =
        // OpenOptions::new().write(true).create(true).truncate(true).open(&
        // data).unwrap(); file.write_all(json.as_bytes()).unwrap();
    }

    #[test]
    fn check_layout_file_links() {
        const DATA_DIR: &str = env!("CARGO_MANIFEST_DIR");
        let mut data_path = PathBuf::from(DATA_DIR);
        data_path.push("data");
        data_path.push("aura_support.ron");

        let mut buf = std::fs::read_to_string(&data_path).unwrap();
        let data: LedSupportFile = ron::from_str(&buf).unwrap();

        data_path.pop();
        data_path.push("layouts");
        data_path.push("loop_prep");

        for config in data.get().iter().rev() {
            if config.layout_name.is_empty() {
                continue;
            }

            buf.clear();

            let layout_file = format!("{}_US.ron", config.layout_name);
            data_path.pop();
            data_path.push(&layout_file);

            let mut file = OpenOptions::new()
                .read(true)
                .open(&data_path)
                .map_err(|e| {
                    panic!(
                        "Error checking {data_path:?} for {} : {e:?}",
                        config.device_name
                    )
                })
                .unwrap();
            #[allow(clippy::verbose_file_reads)]
            if let Err(e) = file.read_to_string(&mut buf) {
                panic!(
                    "Error checking {data_path:?} for {} : {e:?}",
                    config.device_name
                )
            }
            if let Err(e) = ron::from_str::<KeyLayout>(&buf) {
                panic!("Error checking {data_path:?} : {e:?}")
            }
        }
    }
}
