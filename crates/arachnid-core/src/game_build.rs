pub const ORIGINAL: &str = "facb480a78b1d5dfd2cdbba31be89be9cf3cd01e1eadd524de35f23ea436d60d";

pub fn supported(hash: &str) -> bool {
    hash == ORIGINAL
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_original_build_is_supported_after_frame_patch_withdrawal() {
        assert!(supported(ORIGINAL));
        assert!(!supported(
            "033b51e9b4b0a1592733b8140bc14f1e7cfa7c148c6f96a22f473913230ac956"
        ));
        assert!(!supported(&"0".repeat(64)));
    }
}
