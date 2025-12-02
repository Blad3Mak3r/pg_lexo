//! Operations module for lexicographic position generation.
//!
//! This module contains all the core logic for generating and manipulating
//! Base62-encoded lexicographic positions.

/// Base62 character set: 0-9, A-Z, a-z (62 characters)
/// Sorted in ASCII/lexicographic order for proper string comparison
pub const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
pub const BASE: usize = 62;

/// The first character in base62 (index 0)
pub const START_CHAR: char = '0';
/// The last character in base62 (index 61)
pub const END_CHAR: char = 'z';
/// The initial character for first position (index 17 = 'H')
/// Chosen to reduce front spacing while allowing room for prepending
pub const MID_CHAR: char = 'H';

/// Check if a string contains only valid Base62 characters
pub fn is_valid_base62(s: &str) -> bool {
    s.chars().all(|c| BASE62_CHARS.contains(&(c as u8)))
}

/// Get the index of a character in the base62 character set
pub fn char_to_index(c: char) -> Option<usize> {
    BASE62_CHARS.iter().position(|&x| x == c as u8)
}

/// Get the character at a given index in the base62 character set
pub fn index_to_char(idx: usize) -> Option<char> {
    BASE62_CHARS.get(idx).map(|&b| b as char)
}

/// Generate a vector of evenly distributed position strings
pub fn generate_balanced_positions(count: usize) -> Vec<String> {
    if count == 0 {
        return vec![];
    }
    if count == 1 {
        return vec![MID_CHAR.to_string()];
    }

    let mut positions = Vec::with_capacity(count);

    // Distribute positions evenly using fractional approach
    for i in 0..count {
        let fraction = (i as f64 + 0.5) / (count as f64);
        positions.push(fraction_to_position(fraction));
    }

    positions
}

/// Convert a fraction (0.0 to 1.0) to a position string with minimal length
pub fn fraction_to_position(fraction: f64) -> String {
    if fraction <= 0.0 {
        return START_CHAR.to_string();
    }
    if fraction >= 1.0 {
        return END_CHAR.to_string();
    }

    let base = BASE as f64;
    let mut result = String::new();
    let mut remaining = fraction;

    // Generate characters, stopping when we have enough precision
    // or when we've generated a reasonable length (max 6 characters)
    for _ in 0..6 {
        remaining *= base;
        let idx = remaining.floor() as usize;
        let idx = idx.min(BASE - 1);

        if let Some(c) = index_to_char(idx) {
            result.push(c);
        }

        remaining -= idx as f64;

        // Stop if we have enough precision (remaining difference is tiny)
        if remaining < 0.0001 {
            break;
        }
    }

    if result.is_empty() {
        result.push(MID_CHAR);
    }

    result
}

/// Generate a position string after the given string with minimal spacing
pub fn generate_after(s: &str) -> String {
    if s.is_empty() {
        return MID_CHAR.to_string();
    }

    let chars: Vec<char> = s.chars().collect();

    // Try to increment from the rightmost position
    for i in (0..chars.len()).rev() {
        if let Some(idx) = char_to_index(chars[i])
            && idx < BASE - 1
        {
            // Can increment this character
            let mut result: String = chars[..i].iter().collect();
            result.push(index_to_char(idx + 1).unwrap());
            return result;
        }
        // This char is 'z', continue to previous position
    }

    // All characters are 'z', append a character to go after
    // "z" + "0" = "z0" which is lexicographically after "z"
    format!("{}{}", s, START_CHAR)
}

/// Generate a position string before the given string with minimal spacing
///
/// # Panics
/// This function will panic if called with a string consisting entirely of '0' characters,
/// as there is no valid position before the minimum in lexicographic ordering.
pub fn generate_before(s: &str) -> String {
    if s.is_empty() {
        return MID_CHAR.to_string();
    }

    let chars: Vec<char> = s.chars().collect();

    // Try to decrement from the rightmost position
    for i in (0..chars.len()).rev() {
        if let Some(idx) = char_to_index(chars[i])
            && idx > 0
        {
            // Can decrement this character
            let mut result: String = chars[..i].iter().collect();

            // If this is the last character and we can decrement by more than 1
            // just decrement by 1 for minimal spacing
            if i == chars.len() - 1 && idx > 1 {
                result.push(index_to_char(idx - 1).unwrap());
                return result;
            }

            // Otherwise, decrement and add a high character to ensure proper ordering
            result.push(index_to_char(idx - 1).unwrap());
            result.push(END_CHAR);
            return result;
        }
        // This char is '0', continue to previous position
    }

    // All characters are '0' - this is the minimum possible position
    panic!(
        "Cannot generate a position before '{}': this is the minimum possible position",
        s
    );
}

/// Generate a position string between two strings with minimal spacing
pub fn generate_between(before: &str, after: &str) -> String {
    if before.is_empty() && after.is_empty() {
        return MID_CHAR.to_string();
    }
    if before.is_empty() {
        return generate_before(after);
    }
    if after.is_empty() {
        return generate_after(before);
    }

    if before >= after {
        return generate_after(before);
    }

    let before_chars: Vec<char> = before.chars().collect();
    let after_chars: Vec<char> = after.chars().collect();
    let max_len = before_chars.len().max(after_chars.len());

    // Find the first position where characters differ
    for i in 0..max_len {
        let b_char = before_chars.get(i).copied().unwrap_or(START_CHAR);
        let a_char = after_chars.get(i).copied().unwrap_or(END_CHAR);

        let b_idx = char_to_index(b_char).unwrap_or(0);
        let a_idx = char_to_index(a_char).unwrap_or(BASE - 1);

        if b_idx < a_idx {
            let mut result: String = before_chars.iter().take(i).collect();

            // Check if there's room between the characters
            if a_idx - b_idx > 1 {
                // There's at least one character between them
                let mid_idx = (b_idx + a_idx) / 2;
                result.push(index_to_char(mid_idx).unwrap());
                return result;
            }
            // Adjacent characters (e.g., 'A' and 'B')
            // We need to look deeper into the strings
            result.push(b_char);

            // Check if before has more characters at this position
            if i + 1 < before_chars.len() {
                // before has more chars, try to increment from there
                let rest: String = before_chars[i + 1..].iter().collect();
                let after_rest = generate_after(&rest);
                result.push_str(&after_rest);
                return result;
            }
            // before ends here, after continues or also ends
            // Use the middle character to create a position between
            result.push(MID_CHAR);
            return result;
        } else if b_idx == a_idx {
            // Characters are the same, continue to next position
            continue;
        }
    }

    // Strings are equal or before is a prefix of after
    format!("{}{}", before, MID_CHAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_after_minimal() {
        assert_eq!(generate_after("A"), "B");
        assert_eq!(generate_after("0"), "1");
        assert_eq!(generate_after("Z"), "a");
    }

    #[test]
    fn test_generate_before_minimal() {
        assert_eq!(generate_before("B"), "A");
        assert_eq!(generate_before("Z"), "Y");
        assert_eq!(generate_before("a"), "Z");
        assert_eq!(generate_before("1"), "0z");
    }

    #[test]
    fn test_generate_after_overflow() {
        let result = generate_after("z");
        assert!(result > "z".to_string());
        assert_eq!(result, "z0");
    }

    #[test]
    fn test_generate_after_basic() {
        let pos = generate_after("H");
        assert!(pos > "H".to_string());
    }

    #[test]
    fn test_generate_before_basic() {
        let pos = generate_before("H");
        assert!(pos < "H".to_string());
    }

    #[test]
    fn test_generate_between_basic() {
        let pos = generate_between("0", "z");
        assert!(pos > "0".to_string());
        assert!(pos < "z".to_string());
    }

    #[test]
    fn test_generate_between_adjacent() {
        let pos = generate_between("0", "1");
        assert!(pos > "0".to_string());
        assert!(pos < "1".to_string());
    }

    #[test]
    fn test_generate_between_tight() {
        let result = generate_between("A", "C");
        assert_eq!(result, "B");

        let result2 = generate_between("Z", "a");
        assert!(result2 > "Z".to_string());
        assert!(result2 < "a".to_string());
    }

    #[test]
    fn test_generate_between_adjacent_chars() {
        let result = generate_between("A", "B");
        assert!(result > "A".to_string());
        assert!(result < "B".to_string());
        assert!(result.len() <= 2);
    }

    #[test]
    fn test_tight_spacing_sequence() {
        let first = MID_CHAR.to_string();
        let second = generate_after(&first);
        let third = generate_after(&second);

        assert!(second.len() <= 2, "Second position too long: {}", second);
        assert!(third.len() <= 2, "Third position too long: {}", third);
    }

    #[test]
    fn test_sequence_maintains_order() {
        let first = MID_CHAR.to_string();
        let second = generate_after(&first);
        let third = generate_after(&second);
        let fourth = generate_after(&third);

        assert!(first < second);
        assert!(second < third);
        assert!(third < fourth);
    }

    #[test]
    fn test_insert_between_maintains_order() {
        let first = MID_CHAR.to_string();
        let third = generate_after(&first);
        let second = generate_between(&first, &third);

        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn test_multiple_insertions() {
        let first = MID_CHAR.to_string();
        let mut positions: Vec<String> = vec![first];

        for _ in 0..5 {
            let last = positions.last().unwrap();
            positions.push(generate_after(last));
        }

        for i in 0..positions.len() - 1 {
            assert!(
                positions[i] < positions[i + 1],
                "Position {} ({}) should be less than position {} ({})",
                i,
                positions[i],
                i + 1,
                positions[i + 1]
            );
        }
    }

    #[test]
    fn test_insert_at_beginning() {
        let first = MID_CHAR.to_string();
        let before_first = generate_before(&first);

        assert!(before_first < first);
    }

    #[test]
    fn test_generate_after_empty_string() {
        let pos = generate_after("");
        assert_eq!(pos, "H");
    }

    #[test]
    fn test_generate_before_empty_string() {
        let pos = generate_before("");
        assert_eq!(pos, "H");
    }

    #[test]
    fn test_generate_between_empty_strings() {
        let pos = generate_between("", "");
        assert_eq!(pos, "H");
    }

    #[test]
    fn test_generate_between_invalid_order() {
        let pos = generate_between("z", "0");
        assert!(pos > "z".to_string());
    }

    #[test]
    fn test_generate_between_equal_strings() {
        let pos = generate_between("H", "H");
        assert!(pos > "H".to_string());
    }

    #[test]
    fn test_base62_char_conversion() {
        assert_eq!(char_to_index('0'), Some(0));
        assert_eq!(char_to_index('9'), Some(9));
        assert_eq!(char_to_index('A'), Some(10));
        assert_eq!(char_to_index('Z'), Some(35));
        assert_eq!(char_to_index('a'), Some(36));
        assert_eq!(char_to_index('z'), Some(61));

        assert_eq!(index_to_char(0), Some('0'));
        assert_eq!(index_to_char(9), Some('9'));
        assert_eq!(index_to_char(10), Some('A'));
        assert_eq!(index_to_char(35), Some('Z'));
        assert_eq!(index_to_char(36), Some('a'));
        assert_eq!(index_to_char(61), Some('z'));
    }

    #[test]
    fn test_is_valid_base62() {
        assert!(is_valid_base62("H"));
        assert!(is_valid_base62("abc123XYZ"));
        assert!(!is_valid_base62("hello!"));
        assert!(!is_valid_base62("test-value"));
        assert!(is_valid_base62(""));
    }

    #[test]
    fn test_generate_between_same_prefix() {
        let pos = generate_between("AB", "AC");
        assert!(pos > "AB".to_string());
        assert!(pos < "AC".to_string());
    }

    #[test]
    fn test_generate_between_adjacent_with_prefix() {
        let pos = generate_between("A0", "A1");
        assert!(pos > "A0".to_string());
        assert!(pos < "A1".to_string());
    }

    #[test]
    fn test_deep_insertion() {
        let mut positions = vec!["0".to_string(), "1".to_string()];

        for _ in 0..10 {
            let mid = generate_between(&positions[0], &positions[1]);
            assert!(
                mid > positions[0],
                "mid {} should be > {}",
                mid,
                positions[0]
            );
            assert!(
                mid < positions[1],
                "mid {} should be < {}",
                mid,
                positions[1]
            );
            positions.insert(1, mid);
        }
    }

    #[test]
    fn test_generate_between_different_lengths() {
        let pos = generate_between("z", "z1");
        assert!(pos > "z".to_string());
        assert!(pos < "z1".to_string());

        let pos2 = generate_between("A", "AA");
        assert!(pos2 > "A".to_string());
        assert!(pos2 < "AA".to_string());
    }

    #[test]
    #[should_panic(expected = "Cannot generate a position before")]
    fn test_generate_before_minimum_position_panics() {
        let _ = generate_before("0");
    }

    #[test]
    #[should_panic(expected = "Cannot generate a position before")]
    fn test_generate_before_all_zeros_panics() {
        let _ = generate_before("000");
    }

    #[test]
    fn test_generate_between_z_and_z0() {
        let result = generate_between("z", "z0");
        assert!(result > "z".to_string());
    }

    #[test]
    fn test_generate_before_with_trailing_zeros() {
        let pos = generate_before("A0");
        assert!(pos < "A0".to_string());

        let pos2 = generate_before("10");
        assert!(pos2 < "10".to_string());
    }

    #[test]
    fn test_generate_balanced_positions_empty() {
        let positions = generate_balanced_positions(0);
        assert!(positions.is_empty());
    }

    #[test]
    fn test_generate_balanced_positions_single() {
        let positions = generate_balanced_positions(1);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], "H");
    }

    #[test]
    fn test_generate_balanced_positions_multiple() {
        let positions = generate_balanced_positions(5);
        assert_eq!(positions.len(), 5);

        for i in 0..positions.len() - 1 {
            assert!(
                positions[i] < positions[i + 1],
                "Position {} ({}) should be < position {} ({})",
                i,
                positions[i],
                i + 1,
                positions[i + 1]
            );
        }
    }

    #[test]
    fn test_generate_balanced_positions_large() {
        let positions = generate_balanced_positions(100);
        assert_eq!(positions.len(), 100);

        for i in 0..positions.len() - 1 {
            assert!(
                positions[i] < positions[i + 1],
                "Position {} ({}) should be < position {} ({})",
                i,
                positions[i],
                i + 1,
                positions[i + 1]
            );
        }
    }

    #[test]
    fn test_fraction_to_position() {
        let pos_0 = fraction_to_position(0.0);
        let pos_half = fraction_to_position(0.5);
        let pos_1 = fraction_to_position(0.999);

        assert!(pos_0 < pos_half);
        assert!(pos_half < pos_1);
    }
}
