use std::path::Path;

const HEADER_BYTES: usize = 8232;
const PAYLOAD_BYTES: usize = 66320;
const GAME_ID: [u8; 16] = [
    0xef, 0xcf, 0x26, 0xbb, 0xa9, 0x1b, 0x65, 0x44, 0x9d, 0x7c, 0x12, 0xea, 0xf8, 0x13, 0xa3, 0xfe,
];

pub fn read_playtime(source: &Path) -> Option<u64> {
    let image = arachnid_repair::filesystem::read(source, crate::discovery::SAVE_NAME).ok()??;
    playtime_seconds(&image.bytes)
}

/// Reads the native version-19 save statistics, never a process-runtime estimate.
pub fn playtime_seconds(bytes: &[u8]) -> Option<u64> {
    if bytes.len() != HEADER_BYTES + 12 + PAYLOAD_BYTES
        || bytes.get(..4)? != b"RGMH"
        || integer(bytes, 4)? != 1
        || integer(bytes, 8)? != HEADER_BYTES as u32
        || bytes.get(24..40)? != GAME_ID
        || integer(bytes, HEADER_BYTES)? != 1
        || integer(bytes, HEADER_BYTES + 4)? != 0
        || integer(bytes, HEADER_BYTES + 12)? != 19
        || integer(bytes, HEADER_BYTES + 16)? != PAYLOAD_BYTES as u32
        || integer(bytes, HEADER_BYTES + 8)? != crc32fast::hash(&bytes[HEADER_BYTES + 12..])
    {
        return None;
    }
    // The native save serializes the accumulated clock as hours, minutes and fractional seconds.
    let hours = integer(bytes, HEADER_BYTES + 32)?;
    let minutes = integer(bytes, HEADER_BYTES + 36)?;
    let seconds = f32::from_bits(integer(bytes, HEADER_BYTES + 40)?);
    if hours > 1_000_000 || minutes >= 60 || !seconds.is_finite() || !(0.0..60.0).contains(&seconds)
    {
        return None;
    }
    Some(u64::from(hours) * 3600 + u64::from(minutes) * 60 + seconds.floor() as u64)
}

fn integer(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn checksum(bytes: &mut [u8]) {
        set(
            bytes,
            HEADER_BYTES + 8,
            crc32fast::hash(&bytes[HEADER_BYTES + 12..]),
        );
    }

    fn fixture(hours: u32, minutes: u32, seconds: f32) -> Vec<u8> {
        let mut bytes = vec![0; HEADER_BYTES + 12 + PAYLOAD_BYTES];
        bytes[..4].copy_from_slice(b"RGMH");
        set(&mut bytes, 4, 1);
        set(&mut bytes, 8, HEADER_BYTES as u32);
        bytes[24..40].copy_from_slice(&GAME_ID);
        set(&mut bytes, HEADER_BYTES, 1);
        set(&mut bytes, HEADER_BYTES + 12, 19);
        set(&mut bytes, HEADER_BYTES + 16, PAYLOAD_BYTES as u32);
        set(&mut bytes, HEADER_BYTES + 32, hours);
        set(&mut bytes, HEADER_BYTES + 36, minutes);
        set(&mut bytes, HEADER_BYTES + 40, seconds.to_bits());
        checksum(&mut bytes);
        bytes
    }

    #[test]
    fn reads_native_clock_without_day_wrap_or_adding_launcher_time() {
        assert_eq!(playtime_seconds(&fixture(11, 5, 44.98)), Some(39944));
        assert_eq!(playtime_seconds(&fixture(53, 59, 59.9)), Some(194399));
        assert_eq!(playtime_seconds(&fixture(0, 0, 0.0)), Some(0));
    }

    #[test]
    fn rejects_truncation_wrong_game_future_formats_and_torn_payloads() {
        let original = fixture(11, 5, 44.98);
        for size in [0, 4, HEADER_BYTES, original.len() - 1] {
            assert_eq!(playtime_seconds(&original[..size]), None);
        }
        for offset in [
            0,
            4,
            8,
            24,
            HEADER_BYTES,
            HEADER_BYTES + 4,
            HEADER_BYTES + 8,
            HEADER_BYTES + 12,
            HEADER_BYTES + 16,
            HEADER_BYTES + 32,
            original.len() - 1,
        ] {
            let mut bytes = original.clone();
            bytes[offset] ^= 1;
            assert_eq!(playtime_seconds(&bytes), None);
        }
        let mut bytes = original;
        bytes.push(0);
        assert_eq!(playtime_seconds(&bytes), None);
    }

    #[test]
    fn rejects_invalid_clock_even_with_matching_checksum() {
        for (hours, minutes, seconds) in [
            (1_000_001, 0, 0.0),
            (1, 60, 0.0),
            (1, 0, 60.0),
            (1, 0, -1.0),
            (1, 0, f32::NAN),
            (1, 0, f32::INFINITY),
        ] {
            assert_eq!(playtime_seconds(&fixture(hours, minutes, seconds)), None);
        }
    }

    #[test]
    fn reading_a_save_preserves_bytes_and_missing_or_corrupt_files_are_optional() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(read_playtime(directory.path()), None);
        let path = directory.path().join(crate::discovery::SAVE_NAME);
        let bytes = fixture(11, 5, 44.98);
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(read_playtime(directory.path()), Some(39944));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::write(&path, b"incomplete").unwrap();
        assert_eq!(read_playtime(directory.path()), None);
    }
}
