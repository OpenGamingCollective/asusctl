#[cfg(test)]
mod battery_pattern_tests {
    use rog_slash::battery_pattern;

    #[test]
    fn zero_percent_is_all_dark() {
        assert_eq!(battery_pattern(7, 0.0, 255), vec![0u8; 7]);
    }

    #[test]
    fn full_percent_is_all_lit() {
        assert_eq!(battery_pattern(7, 100.0, 255), vec![255u8; 7]);
    }

    #[test]
    fn half_battery_seven_segments() {
        // step = 100/7 ~= 14.286, bracket = floor(50/14.286) = 3
        // -> last 3 segments fully lit, segment index 3 partially lit
        let p = battery_pattern(7, 50.0, 255);
        assert_eq!(
            p,
            vec![
                0, 0, 0, 127, 255, 255, 255
            ]
        );
    }

    #[test]
    fn empty_length_does_not_panic() {
        assert_eq!(battery_pattern(0, 42.0, 255), Vec::<u8>::new());
    }

    #[test]
    fn full_percent_half_brightness() {
        assert_eq!(battery_pattern(7, 100.0, 127), vec![127u8; 7]);
    }

    #[test]
    fn full_percent_no_brightness() {
        assert_eq!(battery_pattern(7, 100.0, 0), vec![0u8; 7]);
    }

    #[test]
    fn half_battery_seven_segments_half_brightness() {
        // step = 100/7 ~= 14.286, bracket = floor(50/14.286) = 3
        // -> last 3 segments fully lit, segment index 3 partially lit
        let p = battery_pattern(7, 50.0, 127);
        assert_eq!(
            p,
            vec![
                0, 0, 0, 63, 127, 127, 127
            ]
        );
    }
}
