/// Offset in milliseconds for sliding effect, the higher the slower.
const SLIDING_TEXT_OFFSET_MILLIS: usize = 500;

/// Get sliding window of text based on elapsed time
pub fn get_sliding_text(
    duration: std::time::Duration,
    full_text: &str,
    window_size: usize,
) -> String {
    if full_text.len() <= window_size {
        // return full text if it fits
        full_text.to_string()
    } else {
        // calculate offset based on elapsed milliseconds
        let elapsed_millis = duration.as_millis() as usize;

        // add +1 to length here, which will be unwrapped as empty space at the end
        let offset = (elapsed_millis / SLIDING_TEXT_OFFSET_MILLIS) % full_text.len();

        // create sliding window by cycling through the text
        // TODO: do this more performant
        let mut result = String::new();
        for i in 0..window_size {
            let idx = (offset + i) % (full_text.len() + 1);
            result.push(full_text.chars().nth(idx).unwrap_or(' '));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    // TODO: sliding text tests
}
