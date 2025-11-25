use pgrx::prelude::*;

::pgrx::pg_module_magic!();

/// Base62 character set: 0-9, A-Z, a-z (62 characters)
/// Sorted in ASCII/lexicographic order for proper string comparison
const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// The first character in base62 (index 0)
const START_CHAR: char = '0';
/// The last character in base62 (index 61)
const END_CHAR: char = 'z';
/// The middle character in base62 (index 31 = 'V')
const MID_CHAR: char = 'V';

/// Get the index of a character in the base62 character set
fn char_to_index(c: char) -> Option<usize> {
    BASE62_CHARS.iter().position(|&x| x == c as u8)
}

/// Get the character at a given index in the base62 character set
fn index_to_char(idx: usize) -> Option<char> {
    BASE62_CHARS.get(idx).map(|&b| b as char)
}

/// Generates a lexicographic position string that comes between two positions.
/// 
/// This function is used to insert items between existing positions in an ordered list.
/// 
/// # Arguments
/// * `before` - The position string before the new position (can be NULL for first position)
/// * `after` - The position string after the new position (can be NULL for last position)
/// 
/// # Returns
/// A new position string that lexicographically falls between `before` and `after`
/// 
/// # Example
/// ```sql
/// SELECT lexical_position_between(NULL, NULL);  -- Returns initial position 'V'
/// SELECT lexical_position_between('V', NULL);   -- Returns position after 'V'
/// SELECT lexical_position_between(NULL, 'V');   -- Returns position before 'V'
/// SELECT lexical_position_between('A', 'Z');    -- Returns midpoint between 'A' and 'Z'
/// ```
#[pg_extern]
fn lexical_position_between(before: Option<&str>, after: Option<&str>) -> String {
    match (before, after) {
        (None, None) => {
            // First position in an empty list
            MID_CHAR.to_string()
        }
        (Some(b), None) => {
            // Position after the last item
            generate_after(b)
        }
        (None, Some(a)) => {
            // Position before the first item
            generate_before(a)
        }
        (Some(b), Some(a)) => {
            // Position between two existing items
            generate_between(b, a)
        }
    }
}

/// Generates the first lexicographic position for a new ordered list.
/// 
/// # Returns
/// The initial position string (middle of base62: 'V')
/// 
/// # Example
/// ```sql
/// SELECT lexical_position_first();  -- Returns 'V'
/// ```
#[pg_extern]
fn lexical_position_first() -> String {
    MID_CHAR.to_string()
}

/// Generates a lexicographic position after the given position.
/// 
/// # Arguments
/// * `current` - The current position string
/// 
/// # Returns
/// A new position string that comes after `current`
/// 
/// # Example
/// ```sql
/// SELECT lexical_position_after('V');  -- Returns a position after 'V'
/// ```
#[pg_extern]
fn lexical_position_after(current: &str) -> String {
    generate_after(current)
}

/// Generates a lexicographic position before the given position.
/// 
/// # Arguments
/// * `current` - The current position string
/// 
/// # Returns
/// A new position string that comes before `current`
/// 
/// # Example
/// ```sql
/// SELECT lexical_position_before('V');  -- Returns a position before 'V'
/// ```
#[pg_extern]
fn lexical_position_before(current: &str) -> String {
    generate_before(current)
}

/// Generate a position string after the given string
fn generate_after(s: &str) -> String {
    // Handle empty string
    if s.is_empty() {
        return MID_CHAR.to_string();
    }
    
    let chars: Vec<char> = s.chars().collect();
    
    // Try to increment the last character using base62
    if let Some(&last_char) = chars.last() {
        if let Some(last_idx) = char_to_index(last_char) {
            let end_idx = BASE62_CHARS.len() - 1; // 61
            if last_idx < end_idx {
                // Calculate midpoint between current and end
                let mid_idx = (last_idx + end_idx) / 2;
                if mid_idx > last_idx {
                    if let Some(mid) = index_to_char(mid_idx) {
                        let mut result: String = chars[..chars.len() - 1].iter().collect();
                        result.push(mid);
                        return result;
                    }
                }
            }
        }
    }
    
    // Append a middle character to extend the string
    format!("{}{}", s, MID_CHAR)
}

/// Generate a position string before the given string
fn generate_before(s: &str) -> String {
    // Handle empty string
    if s.is_empty() {
        return MID_CHAR.to_string();
    }
    
    let chars: Vec<char> = s.chars().collect();
    
    // Try to decrement the last character using base62
    if let Some(&last_char) = chars.last() {
        if let Some(last_idx) = char_to_index(last_char) {
            if last_idx > 0 {
                // Calculate midpoint between start and current
                let mid_idx = last_idx / 2;
                if mid_idx < last_idx {
                    if let Some(mid) = index_to_char(mid_idx) {
                        let mut result: String = chars[..chars.len() - 1].iter().collect();
                        result.push(mid);
                        return result;
                    }
                }
            }
        }
    }
    
    // If we can't decrement, extend with a character before the last one
    let prefix: String = chars[..chars.len() - 1].iter().collect();
    format!("{}{}{}", prefix, START_CHAR, MID_CHAR)
}

/// Generate a position string between two strings
/// If before >= after, this generates a position after 'before' as a fallback
fn generate_between(before: &str, after: &str) -> String {
    // Handle empty strings
    if before.is_empty() && after.is_empty() {
        return MID_CHAR.to_string();
    }
    if before.is_empty() {
        return generate_before(after);
    }
    if after.is_empty() {
        return generate_after(before);
    }
    
    // If before >= after, fall back to generating after 'before'
    if before >= after {
        return generate_after(before);
    }
    
    let before_chars: Vec<char> = before.chars().collect();
    let after_chars: Vec<char> = after.chars().collect();
    
    let max_len = before_chars.len().max(after_chars.len());
    let mut result = String::new();
    
    for i in 0..max_len {
        let b_char = before_chars.get(i).copied().unwrap_or(START_CHAR);
        let a_char = after_chars.get(i).copied().unwrap_or(END_CHAR);
        
        let b_idx = char_to_index(b_char).unwrap_or(0);
        let a_idx = char_to_index(a_char).unwrap_or(BASE62_CHARS.len() - 1);
        
        if b_idx < a_idx {
            // Found a position where we can insert a character between
            let mid_idx = (b_idx + a_idx) / 2;
            if mid_idx > b_idx && mid_idx < a_idx {
                if let Some(mid) = index_to_char(mid_idx) {
                    result.push(mid);
                    return result;
                }
            } else if mid_idx > b_idx {
                // mid_idx == a_idx, need to extend
                if let Some(c) = index_to_char(b_idx) {
                    result.push(c);
                }
                // Continue to find a spot
            } else {
                // mid_idx == b_idx, push b_char and extend
                if let Some(c) = index_to_char(b_idx) {
                    result.push(c);
                }
            }
        } else {
            if let Some(c) = index_to_char(b_idx) {
                result.push(c);
            }
        }
    }
    
    // If we couldn't find a gap, extend with a middle character
    result.push(MID_CHAR);
    result
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_lexical_position_first() {
        let pos = crate::lexical_position_first();
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_lexical_position_between_null_null() {
        let pos = crate::lexical_position_between(None, None);
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_lexical_position_after() {
        let pos = crate::lexical_position_after("V");
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_lexical_position_before() {
        let pos = crate::lexical_position_before("V");
        assert!(pos < "V".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_with_before() {
        let pos = crate::lexical_position_between(Some("0"), None);
        assert!(pos > "0".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_with_after() {
        let pos = crate::lexical_position_between(None, Some("z"));
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_two_values() {
        let pos = crate::lexical_position_between(Some("0"), Some("z"));
        assert!(pos > "0".to_string());
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_ordering_sequence() {
        // Test that we can create a sequence of ordered positions
        let first = crate::lexical_position_first();
        let second = crate::lexical_position_after(&first);
        let third = crate::lexical_position_after(&second);
        
        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_insert_between_sequence() {
        // Test inserting between existing positions
        let first = crate::lexical_position_first();
        let third = crate::lexical_position_after(&first);
        let second = crate::lexical_position_between(Some(&first), Some(&third));
        
        assert!(first < second);
        assert!(second < third);
    }
}

/// Standard Rust unit tests that don't require PostgreSQL
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_generate_first() {
        assert_eq!(lexical_position_first(), "V");
    }

    #[test]
    fn test_generate_after_basic() {
        let pos = generate_after("V");
        assert!(pos > "V".to_string());
    }

    #[test]
    fn test_generate_before_basic() {
        let pos = generate_before("V");
        assert!(pos < "V".to_string());
    }

    #[test]
    fn test_generate_between_basic() {
        let pos = generate_between("0", "z");
        assert!(pos > "0".to_string());
        assert!(pos < "z".to_string());
    }

    #[test]
    fn test_generate_between_adjacent() {
        // When characters are adjacent in base62, we need to extend
        let pos = generate_between("0", "1");
        assert!(pos > "0".to_string());
        assert!(pos < "1".to_string());
    }

    #[test]
    fn test_sequence_maintains_order() {
        let first = lexical_position_first();
        let second = generate_after(&first);
        let third = generate_after(&second);
        let fourth = generate_after(&third);
        
        assert!(first < second);
        assert!(second < third);
        assert!(third < fourth);
    }

    #[test]
    fn test_insert_between_maintains_order() {
        let first = lexical_position_first();
        let third = generate_after(&first);
        let second = generate_between(&first, &third);
        
        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn test_multiple_insertions() {
        // Simulate multiple insertions
        let mut positions = vec![lexical_position_first()];
        
        // Add 5 positions after
        for _ in 0..5 {
            let last = positions.last().unwrap();
            positions.push(generate_after(last));
        }
        
        // Verify all are in order
        for i in 0..positions.len() - 1 {
            assert!(positions[i] < positions[i + 1], 
                "Position {} ({}) should be less than position {} ({})", 
                i, positions[i], i + 1, positions[i + 1]);
        }
    }

    #[test]
    fn test_insert_at_beginning() {
        let first = lexical_position_first();
        let before_first = generate_before(&first);
        
        assert!(before_first < first);
    }

    #[test]
    fn test_lexical_position_between_function() {
        // Test the public API
        assert_eq!(lexical_position_between(None, None), "V");
        
        let after_v = lexical_position_between(Some("V"), None);
        assert!(after_v > "V".to_string());
        
        let before_v = lexical_position_between(None, Some("V"));
        assert!(before_v < "V".to_string());
        
        let between = lexical_position_between(Some("0"), Some("z"));
        assert!(between > "0".to_string());
        assert!(between < "z".to_string());
    }

    // Edge case tests
    #[test]
    fn test_generate_after_empty_string() {
        let pos = generate_after("");
        assert_eq!(pos, "V");
    }

    #[test]
    fn test_generate_before_empty_string() {
        let pos = generate_before("");
        assert_eq!(pos, "V");
    }

    #[test]
    fn test_generate_between_empty_strings() {
        let pos = generate_between("", "");
        assert_eq!(pos, "V");
    }

    #[test]
    fn test_generate_between_before_empty() {
        let pos = generate_between("", "z");
        assert!(pos < "z".to_string());
    }

    #[test]
    fn test_generate_between_after_empty() {
        let pos = generate_between("0", "");
        assert!(pos > "0".to_string());
    }

    #[test]
    fn test_generate_between_invalid_order() {
        // When before >= after, should return position after 'before'
        let pos = generate_between("z", "0");
        assert!(pos > "z".to_string());
    }

    #[test]
    fn test_generate_between_equal_strings() {
        // When before == after, should return position after 'before'
        let pos = generate_between("V", "V");
        assert!(pos > "V".to_string());
    }

    // Base62 specific tests
    #[test]
    fn test_base62_char_conversion() {
        // Test that char_to_index and index_to_char work correctly
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
    fn test_base62_full_range() {
        // Test ordering across the full base62 range
        let start = generate_after("0");
        let end = generate_before("z");
        
        assert!(start > "0".to_string());
        assert!(end < "z".to_string());
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
