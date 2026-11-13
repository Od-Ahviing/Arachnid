use arachnid_core::{AppError, AppResult};
use sha2::{Digest, Sha256};

pub const ORIGINAL_SHA256: &str =
    "738e1c71cee12f7175bf9e4cccc6ec36e3b7104b0ba203328144c5ac85469999";
pub const ORIGINAL_BYTES: usize = 2_079_232;
pub const ADAPTED_SHA256: &str = "d9fa9d6fd573c9bf693481323fc7de432804fbbcd5a8029b967941a7b1eddaf8";
pub const DXVK_FILE: &str = "ar_dxvk.dll";
const CALL_OFFSET: usize = 0x52d4d;
const NAME_OFFSET: usize = 0x1ae2cc;
const RELOCATION_OFFSET: usize = 0x1f0f46;
const ORIGINAL_CALL: &[u8] = &[0xff, 0x15, 0x4c, 0x70, 0x2b, 0x10];
// At this call site EAX points to the buffer and the stdcall arguments occupy 8 bytes.
pub const LOCAL_BUFFER_CODE: &[u8] = &[0xc6, 0x00, 0x00, 0x83, 0xc4, 0x08];
const ORIGINAL_NAME: &[u8] = b"\\d3d9.dll\0\0\0";
const LOCAL_NAME: &[u8] = b"ar_dxvk.dll\0";

/// Adapts only the user-provided, statically inspected Fix. No game or system binary is accepted.
pub fn adapt(original: &[u8]) -> AppResult<Vec<u8>> {
    if original.len() != ORIGINAL_BYTES || digest(original) != ORIGINAL_SHA256 {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    if &original[CALL_OFFSET..CALL_OFFSET + ORIGINAL_CALL.len()] != ORIGINAL_CALL
        || &original[NAME_OFFSET..NAME_OFFSET + ORIGINAL_NAME.len()] != ORIGINAL_NAME
    {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    let mut candidate = original.to_vec();
    candidate[CALL_OFFSET..CALL_OFFSET + LOCAL_BUFFER_CODE.len()]
        .copy_from_slice(LOCAL_BUFFER_CODE);
    candidate[NAME_OFFSET..NAME_OFFSET + LOCAL_NAME.len()].copy_from_slice(LOCAL_NAME);
    if original[RELOCATION_OFFSET..RELOCATION_OFFSET + 2] != [0x4f, 0x39] {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    // The removed absolute IAT operand must no longer be relocated when ASLR rebases the DLL.
    candidate[RELOCATION_OFFSET..RELOCATION_OFFSET + 2].copy_from_slice(&[0x4f, 0x09]);
    if digest(&candidate) != ADAPTED_SHA256 {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    Ok(candidate)
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn restore(candidate: &[u8]) -> AppResult<Vec<u8>> {
    if candidate.len() != ORIGINAL_BYTES || digest(candidate) != ADAPTED_SHA256 {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    let mut original = candidate.to_vec();
    original[CALL_OFFSET..CALL_OFFSET + ORIGINAL_CALL.len()].copy_from_slice(ORIGINAL_CALL);
    original[NAME_OFFSET..NAME_OFFSET + ORIGINAL_NAME.len()].copy_from_slice(ORIGINAL_NAME);
    original[RELOCATION_OFFSET..RELOCATION_OFFSET + 2].copy_from_slice(&[0x4f, 0x39]);
    if digest(&original) != ORIGINAL_SHA256 {
        return Err(AppError::new("fixAdapterUnsupported"));
    }
    Ok(original)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_every_unidentified_binary_without_partial_changes() {
        for original in [vec![], vec![0; ORIGINAL_BYTES], vec![0; ORIGINAL_BYTES + 1]] {
            let before = digest(&original);
            assert_eq!(adapt(&original).unwrap_err().code, "fixAdapterUnsupported");
            assert_eq!(
                restore(&original).unwrap_err().code,
                "fixAdapterUnsupported"
            );
            assert_eq!(digest(&original), before);
        }
    }
}
