use std::time::Duration;

/// Offset in milliseconds for sliding effect, the higher the slower.
const SLIDING_TEXT_OFFSET_MILLIS: usize = 500;

/// Get sliding window of text based on elapsed time
pub fn get_sliding_text(duration: Duration, full_text: &str, window_size: usize) -> String {
    if full_text.len() <= window_size {
        // return full text if window is larger than text
        full_text.to_string()
    } else {
        // calculate offset based on elapsed milliseconds
        let elapsed_millis = duration.as_millis() as usize;
        let offset = (elapsed_millis / SLIDING_TEXT_OFFSET_MILLIS) % full_text.len();

        // create sliding window by cycling through the text
        format!("{} {}", &full_text[offset..], &full_text[..offset])
            .chars()
            .take(window_size)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider() {
        // ensure text slides correctly w.r.t duration
        let text = "abc";
        let x = Duration::from_millis(SLIDING_TEXT_OFFSET_MILLIS as u64);

        // smaller window than text length
        // (should cycle-back at text length)
        assert_eq!(get_sliding_text(x * 0, text, 2), "ab");
        assert_eq!(get_sliding_text(x * 1, text, 2), "bc");
        assert_eq!(get_sliding_text(x * 2, text, 2), "c ");
        assert_eq!(get_sliding_text(x * 3, text, 2), "ab");

        // exact window size equals text length (should not slide at all)
        assert_eq!(get_sliding_text(x * 0, text, 3), "abc");
        assert_eq!(get_sliding_text(x * 1, text, 3), "abc");
        assert_eq!(get_sliding_text(x * 2, text, 3), "abc");

        // larger window than text length (should not slide at all)
        assert_eq!(get_sliding_text(x * 0, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 1, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 2, text, 5), "abc");
        assert_eq!(get_sliding_text(x * 3, text, 5), "abc");
    }
}
