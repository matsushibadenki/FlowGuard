//! Read only a bounded Mach-O header; signature/identity checks remain mandatory separately.
pub fn contains(header: &[u8], size: u64, arch: &str) -> bool {
    let cpu = match arch {
        "aarch64" => 0x0100000c,
        "x86_64" => 0x01000007,
        _ => return false,
    };
    if header.len() < 8 {
        return false;
    }
    let be = |at: usize| {
        header
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
    };
    let le = |at: usize| {
        header
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
    };
    match &header[..4] {
        [0xcf, 0xfa, 0xed, 0xfe] => return header.len() >= 32 && size >= 32 && le(4) == Some(cpu),
        [0xfe, 0xed, 0xfa, 0xcf] => return header.len() >= 32 && size >= 32 && be(4) == Some(cpu),
        _ => {}
    }
    let (little, wide) = match &header[..4] {
        [0xca, 0xfe, 0xba, 0xbe] => (false, false),
        [0xbe, 0xba, 0xfe, 0xca] => (true, false),
        [0xca, 0xfe, 0xba, 0xbf] => (false, true),
        [0xbf, 0xba, 0xfe, 0xca] => (true, true),
        _ => return false,
    };
    let u32at = |at| if little { le(at) } else { be(at) };
    let Some(count) = u32at(4).filter(|n| (1..=16).contains(n)) else {
        return false;
    };
    let stride = if wide { 32 } else { 20 };
    let end = 8 + stride * count as usize;
    if header.len() < end {
        return false;
    }
    let mut found = false;
    for i in 0..count as usize {
        let at = 8 + i * stride;
        let read64 = |at: usize| -> Option<u64> {
            let bytes = header.get(at..at + 8)?.try_into().ok()?;
            Some(if little {
                u64::from_le_bytes(bytes)
            } else {
                u64::from_be_bytes(bytes)
            })
        };
        let (offset, length) = if wide {
            (read64(at + 8), read64(at + 16))
        } else {
            (u32at(at + 8).map(u64::from), u32at(at + 12).map(u64::from))
        };
        let (Some(offset), Some(length)) = (offset, length) else {
            return false;
        };
        if offset < end as u64 || length == 0 || offset.checked_add(length).is_none_or(|n| n > size)
        {
            return false;
        }
        found |= u32at(at) == Some(cpu);
    }
    found
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thin_fat_and_wrong_architectures_are_checked() {
        let mut thin = vec![0; 32];
        thin[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        thin[4..8].copy_from_slice(&0x0100000cu32.to_le_bytes());
        assert!(contains(&thin, 32, "aarch64"));
        assert!(!contains(&thin, 32, "x86_64"));
        assert!(!contains(&thin[..8], 8, "aarch64"));
        let mut fat = vec![0; 48];
        fat[..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        fat[4..8].copy_from_slice(&2u32.to_be_bytes());
        for (i, cpu) in [0x0100000cu32, 0x01000007].iter().enumerate() {
            let at = 8 + i * 20;
            fat[at..at + 4].copy_from_slice(&cpu.to_be_bytes());
            fat[at + 8..at + 12].copy_from_slice(&(48 + i as u32 * 32).to_be_bytes());
            fat[at + 12..at + 16].copy_from_slice(&32u32.to_be_bytes());
        }
        assert!(contains(&fat, 112, "aarch64"));
        assert!(contains(&fat, 112, "x86_64"));
        assert!(!contains(&fat, 60, "aarch64"));
        assert!(!contains(b"test executable", 15, "aarch64"));
    }
}
