use log::{error, trace};
use serde::{Deserialize, Serialize};
use udev::Device;
#[cfg(feature = "dbus")]
use zbus::zvariant::Type;

use crate::FanCurvePU;
use crate::error::ProfileError;

pub(crate) fn pwm_str(fan: char, index: usize) -> String {
    format!("pwm{fan}_auto_point{}_pwm", index + 1)
}

pub(crate) fn temp_str(fan: char, index: usize) -> String {
    format!("pwm{fan}_auto_point{}_temp", index + 1)
}

#[cfg_attr(feature = "dbus", derive(Type))]
#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct CurveData {
    pub fan: FanCurvePU,
    pub pwm: [u8; 8],
    pub temp: [u8; 8],
    pub enabled: bool,
}

impl From<&CurveData> for String {
    fn from(c: &CurveData) -> Self {
        format!(
            "{:?}: enabled: {}, {}c:{}%,{}c:{}%,{}c:{}%,{}c:{}%,{}c:{}%,{}c:{}%,{}c:{}%,{}c:{}%",
            c.fan,
            c.enabled,
            c.temp[0],
            (c.pwm[0] as u32) * 100 / 255,
            c.temp[1],
            (c.pwm[1] as u32) * 100 / 255,
            c.temp[2],
            (c.pwm[2] as u32) * 100 / 255,
            c.temp[3],
            (c.pwm[3] as u32) * 100 / 255,
            c.temp[4],
            (c.pwm[4] as u32) * 100 / 255,
            c.temp[5],
            (c.pwm[5] as u32) * 100 / 255,
            c.temp[6],
            (c.pwm[6] as u32) * 100 / 255,
            c.temp[7],
            (c.pwm[7] as u32) * 100 / 255,
        )
    }
}

impl std::str::FromStr for CurveData {
    type Err = ProfileError;

    /// Parse a string to the correct values that the fan curve kernel driver
    /// expects. The returned `CurveData` is not enabled by default.
    ///
    /// If the fan curve is given with percentage char '%' then the fan power
    /// values are converted otherwise the expected fan power range is
    /// 0-255.
    ///
    /// Temperature range is 0-255 in degrees C. You don't want to be setting
    /// over 100.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut temp = [0u8; 8];
        let mut pwm = [0u8; 8];
        let mut temp_prev = 0;
        let mut pwm_prev = 0;
        let mut percentages = false;

        if input.split(',').count() < 8 {
            return Err(ProfileError::NotEnoughPoints);
        }

        for (index, value) in input.split(',').enumerate() {
            for (select, num) in value.splitn(2, ['c', ':']).enumerate() {
                if num.contains('%') {
                    percentages = true;
                }
                let r = num.trim_matches(|c| c == 'c' || c == ':' || c == '%');
                let r = r.parse::<u8>().map_err(ProfileError::ParseFanCurveDigit)?;

                if select == 0 {
                    if temp_prev > r {
                        return Err(ProfileError::ParseFanCurvePrevHigher(
                            "temperature", temp_prev, r,
                        ));
                    }
                    temp_prev = r;
                    temp[index] = r;
                } else {
                    let mut p = r;
                    if percentages {
                        if r > 100 {
                            return Err(ProfileError::ParseFanCurvePercentOver100(r));
                        }
                        p = (p as f32 * 2.55).round() as u8;
                    }
                    if pwm_prev > p {
                        return Err(ProfileError::ParseFanCurvePrevHigher(
                            "percentage", pwm_prev, p,
                        ));
                    }
                    pwm_prev = p;
                    pwm[index] = p;
                }
            }
        }
        Ok(Self {
            fan: FanCurvePU::CPU,
            pwm,
            temp,
            enabled: false,
        })
    }
}

impl CurveData {
    pub fn set_fan(&mut self, fan: FanCurvePU) {
        self.fan = fan;
    }

    fn set_val_from_attr(tmp: &str, device: &Device, buf: &mut [u8; 8]) {
        if let Some(n) = tmp.chars().nth(15) {
            let i = n.to_digit(10).unwrap() as usize;
            let d = device.attribute_value(tmp).unwrap();
            let d: u8 = d.to_string_lossy().parse().unwrap();
            buf[i - 1] = d;
        }
    }

    pub fn read_from_device(&mut self, device: &Device) {
        for attr in device.attributes() {
            let tmp = attr.name().to_string_lossy();
            let pwm_num: char = self.fan.into();
            let pwm = format!("pwm{pwm_num}");
            if tmp.starts_with(&pwm) && tmp.ends_with("_temp") {
                Self::set_val_from_attr(tmp.as_ref(), device, &mut self.temp);
            }
            if tmp.starts_with(&pwm) && tmp.ends_with("_pwm") {
                Self::set_val_from_attr(tmp.as_ref(), device, &mut self.pwm);
            }
        }
    }

    /// Write this curve to the device fan specified by `self.fan`
    pub fn write_to_device(&self, device: &mut Device) -> std::io::Result<()> {
        let pwm_num: char = self.fan.into();
        let enable = if self.enabled { '1' } else { '2' };

        for (index, out) in self.pwm.iter().enumerate() {
            let pwm = pwm_str(pwm_num, index);
            trace!("writing {pwm}");
            device.set_attribute_value(&pwm, out.to_string())?;
        }

        for (index, out) in self.temp.iter().enumerate() {
            let temp = temp_str(pwm_num, index);
            trace!("writing {temp}");
            device.set_attribute_value(&temp, out.to_string())?;
        }

        // Enable must be done *after* all points are written pwm3_enable
        device
            .set_attribute_value(format!("pwm{pwm_num}_enable"), enable.to_string())
            .map_err(|e| error!("Failed to set pwm{pwm_num}_enable to {enable}: {e:?}"))
            .ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn curve_data_from_str_to_str() {
        let mut curve =
            CurveData::from_str("30c:1%,49c:2%,59c:3%,69c:4%,79c:31%,89c:49%,99c:56%,109c:58%")
                .unwrap();
        curve.enabled = true;
        assert_eq!(curve.fan, FanCurvePU::CPU);
        assert_eq!(
            curve.temp,
            [
                30, 49, 59, 69, 79, 89, 99, 109
            ]
        );
        assert_eq!(
            curve.pwm,
            [
                3, 5, 8, 10, 79, 125, 143, 148
            ]
        );

        let string: String = (&curve).into();
        // End result is slightly different due to type conversions and rounding errors
        assert_eq!(
            string.as_str(),
            "CPU: enabled: true, 30c:1%,49c:1%,59c:3%,69c:3%,79c:30%,89c:49%,99c:56%,109c:58%"
        );

        let curve = CurveData::from_str("30c:1%,49c:2%,59c:3%,69c:4%,79c:31%,89c:49%,99c:56%");

        assert!(curve.is_err());
    }

    #[test]
    fn curve_data_from_str_simple() {
        let curve = CurveData::from_str("30:1,49:2,59:3,69:4,79:31,89:49,99:56,109:58").unwrap();
        assert_eq!(curve.fan, FanCurvePU::CPU);
        assert_eq!(
            curve.temp,
            [
                30, 49, 59, 69, 79, 89, 99, 109
            ]
        );
        assert_eq!(
            curve.pwm,
            [
                1, 2, 3, 4, 31, 49, 56, 58
            ]
        );
    }

    #[test]
    fn curve_data_from_str_invalid_pwm() {
        let curve =
            CurveData::from_str("30c:4%,49c:2%,59c:3%,69c:4%,79c:31%,89c:49%,99c:56%,109c:58%");
        assert!(&curve.is_err());
        assert!(matches!(
            curve,
            Err(ProfileError::ParseFanCurvePrevHigher(_, _, _))
        ));
    }

    #[test]
    fn check_pwm_str() {
        assert_eq!(pwm_str('1', 0), "pwm1_auto_point1_pwm");
        assert_eq!(pwm_str('1', 4), "pwm1_auto_point5_pwm");
        assert_eq!(pwm_str('1', 7), "pwm1_auto_point8_pwm");
    }

    #[test]
    fn check_temp_str() {
        assert_eq!(temp_str('1', 0), "pwm1_auto_point1_temp");
        assert_eq!(temp_str('1', 4), "pwm1_auto_point5_temp");
        assert_eq!(temp_str('1', 7), "pwm1_auto_point8_temp");
    }

    // #[test]
    // fn set_to_string() {
    //     let set = FanCurveSet::default();
    //     let string = String::from(&set);
    //     assert_eq!(
    //         string.as_str(),
    //         "Enabled: false, CPU:
    // 0c:0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%, GPU: \          0c:
    // 0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%,0c:0%"     );
    // }
}
