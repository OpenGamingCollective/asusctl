#[cfg(test)]
mod battery_pattern_tests {
    use rog_slash::battery_pattern;

    #[test]
    fn zero_percent_is_all_dark() {
        let mut segments = [0u8; 7];
        battery_pattern(&mut segments, 0.0, 255);
        assert_eq!(segments, [0u8; 7]);
    }

    #[test]
    fn full_percent_is_all_lit() {
        let mut segments = [0u8; 7];
        battery_pattern(&mut segments, 100.0, 255);
        assert_eq!(segments, [255u8; 7]);
    }

    #[test]
    fn half_battery_seven_segments() {
        // step = 100/7 ~= 14.286, bracket = floor(50/14.286) = 3
        // -> last 3 segments fully lit, segment index 3 partially lit
        let mut segments = [0u8; 7];
        battery_pattern(&mut segments, 50.0, 255);
        assert_eq!(
            segments,
            [
                0, 0, 0, 127, 255, 255, 255
            ]
        );
    }

    #[test]
    fn empty_length_does_not_panic() {
        let mut segments: [u8; 0] = [];
        battery_pattern(&mut segments, 42.0, 255);
        assert_eq!(segments, [] as [u8; 0]);
    }

    #[test]
    fn full_percent_half_brightness() {
        let mut segments = [0u8; 7];
        battery_pattern(&mut segments, 100.0, 127);
        assert_eq!(segments, [127u8; 7]);
    }

    #[test]
    fn full_percent_no_brightness() {
        let mut segments = [1u8; 7];
        battery_pattern(&mut segments, 100.0, 0);
        assert_eq!(segments, [0u8; 7]);
    }

    #[test]
    fn half_battery_seven_segments_half_brightness() {
        // step = 100/7 ~= 14.286, bracket = floor(50/14.286) = 3
        // -> last 3 segments fully lit, segment index 3 partially lit
        let mut segments = [0u8; 7];
        battery_pattern(&mut segments, 50.0, 127);
        assert_eq!(
            segments,
            [
                0, 0, 0, 63, 127, 127, 127
            ]
        );
    }
}
