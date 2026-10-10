//! Safe parser for `FILE_NAMES_INFORMATION` buffers returned by Windows.
//!
//! Kept independent from Windows bindings so malformed record handling is
//! tested on every host. The caller converts the returned UTF-16 units into
//! platform-native names only after this boundary validates the layout.

use std::io;

const HEADER_BYTES: usize = 12;

/// Shared across every query page and descendant in a cleanup traversal.
pub(super) struct DirectoryBudget {
    entries_left: usize,
    name_bytes_left: usize,
}

impl DirectoryBudget {
    pub(super) fn new(entries: usize, name_bytes: usize) -> Self {
        Self {
            entries_left: entries,
            name_bytes_left: name_bytes,
        }
    }

    pub(super) fn parse_page(&mut self, buffer: &[u8]) -> io::Result<Vec<Vec<u16>>> {
        let names = parse_names(buffer, self.entries_left)?;
        let name_bytes = names.iter().try_fold(0_usize, |total, name| {
            name.len().checked_mul(2)?.checked_add(total)
        });
        let remaining_bytes = name_bytes
            .and_then(|bytes| self.name_bytes_left.checked_sub(bytes))
            .ok_or_else(|| invalid("scratch directory name byte limit exceeded"))?;
        self.entries_left -= names.len();
        self.name_bytes_left = remaining_bytes;
        Ok(names)
    }
}

/// Parses a complete `NtQueryDirectoryFile(FileNamesInformation)` result.
///
/// `NextEntryOffset` is four-byte aligned. The final record has offset zero
/// and ends at the supplied byte count. Empty and malformed names are rejected
/// before any caller attempts a filesystem operation.
fn parse_names(buffer: &[u8], max_entries: usize) -> io::Result<Vec<Vec<u16>>> {
    let mut names = Vec::new();
    let mut offset = 0usize;
    while offset < buffer.len() {
        if buffer.len() - offset < HEADER_BYTES {
            return Err(invalid("short directory entry"));
        }
        let next = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap()) as usize;
        let name_bytes = u32::from_le_bytes(
            buffer[offset + 8..offset + HEADER_BYTES]
                .try_into()
                .unwrap(),
        ) as usize;
        let record_end = if next == 0 {
            buffer.len()
        } else {
            if !next.is_multiple_of(4) || next < HEADER_BYTES {
                return Err(invalid("invalid directory entry offset"));
            }
            offset
                .checked_add(next)
                .filter(|end| buffer.len().saturating_sub(*end) >= HEADER_BYTES)
                .ok_or_else(|| invalid("invalid directory entry offset"))?
        };
        let name_start = offset + HEADER_BYTES;
        let Some(name_end) = name_start.checked_add(name_bytes) else {
            return Err(invalid("invalid directory entry name"));
        };
        if name_bytes == 0 || !name_bytes.is_multiple_of(2) || name_end > record_end {
            return Err(invalid("invalid directory entry name"));
        }
        names.push(
            buffer[name_start..name_end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect(),
        );
        if names.len() > max_entries {
            return Err(invalid("scratch directory entry limit exceeded"));
        }
        if next == 0 {
            break;
        }
        offset = record_end;
    }
    Ok(names)
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_four_byte_padded_records() {
        // The first record occupies 20 bytes. That is valid four-byte padding
        // and deliberately disproves an incorrect eight-byte parser rule.
        let mut records = vec![0_u8; 34];
        records[0..4].copy_from_slice(&20_u32.to_le_bytes());
        records[8..12].copy_from_slice(&2_u32.to_le_bytes());
        records[12..14].copy_from_slice(&(b'a' as u16).to_le_bytes());
        records[20..24].copy_from_slice(&0_u32.to_le_bytes());
        records[28..32].copy_from_slice(&2_u32.to_le_bytes());
        records[32..34].copy_from_slice(&(b'b' as u16).to_le_bytes());
        assert_eq!(
            parse_names(&records, 2).unwrap(),
            [vec![b'a' as u16], vec![b'b' as u16]]
        );
    }

    #[test]
    fn rejects_zero_odd_and_out_of_bounds_lengths() {
        assert!(parse_names(&[0; 12], 1).is_err());
        let mut odd_name = vec![0_u8; 13];
        odd_name[8..12].copy_from_slice(&1_u32.to_le_bytes());
        assert!(parse_names(&odd_name, 1).is_err());
        let mut out_of_bounds_offset = vec![0_u8; 16];
        out_of_bounds_offset[0..4].copy_from_slice(&20_u32.to_le_bytes());
        out_of_bounds_offset[8..12].copy_from_slice(&2_u32.to_le_bytes());
        assert!(parse_names(&out_of_bounds_offset, 1).is_err());
    }

    #[test]
    fn enforces_the_entry_limit() {
        let mut records = vec![0_u8; 34];
        records[0..4].copy_from_slice(&20_u32.to_le_bytes());
        records[8..12].copy_from_slice(&2_u32.to_le_bytes());
        records[12..14].copy_from_slice(&(b'a' as u16).to_le_bytes());
        records[28..32].copy_from_slice(&2_u32.to_le_bytes());
        records[32..34].copy_from_slice(&(b'b' as u16).to_le_bytes());
        assert!(parse_names(&records, 1).is_err());
    }

    #[test]
    fn continuation_requires_another_complete_record() {
        let mut record = vec![0_u8; 16];
        record[0..4].copy_from_slice(&16_u32.to_le_bytes());
        record[8..12].copy_from_slice(&2_u32.to_le_bytes());
        record[12..14].copy_from_slice(&(b'a' as u16).to_le_bytes());
        assert!(parse_names(&record, 2).is_err());
    }

    fn single_name_record() -> Vec<u8> {
        let mut record = vec![0_u8; 14];
        record[8..12].copy_from_slice(&2_u32.to_le_bytes());
        record[12..14].copy_from_slice(&(b'a' as u16).to_le_bytes());
        record
    }

    #[test]
    fn entry_budget_accumulates_across_query_pages() {
        let record = single_name_record();
        let mut budget = DirectoryBudget::new(2, 100);
        assert!(budget.parse_page(&record).is_ok());
        assert!(budget.parse_page(&record).is_ok());
        assert!(budget.parse_page(&record).is_err());
    }

    #[test]
    fn filename_byte_budget_accumulates_across_query_pages() {
        let record = single_name_record();
        let mut budget = DirectoryBudget::new(100, 3);
        assert!(budget.parse_page(&record).is_ok());
        assert!(budget.parse_page(&record).is_err());
    }
}
