use pgrx::prelude::*;

::pgrx::pg_module_magic!();

/// The default starting character for lexicographic ordering
const START_CHAR: char = 'a';
/// The ending character for lexicographic ordering
const END_CHAR: char = 'z';
/// The middle character for initial positions
const MID_CHAR: char = 'n';

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
/// SELECT lexical_position_between(NULL, NULL);  -- Returns initial position
/// SELECT lexical_position_between('n', NULL);   -- Returns position after 'n'
/// SELECT lexical_position_between(NULL, 'n');   -- Returns position before 'n'
/// SELECT lexical_position_between('a', 'c');    -- Returns 'b'
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
/// The initial position string (middle of the alphabet)
/// 
/// # Example
/// ```sql
/// SELECT lexical_position_first();  -- Returns 'n'
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
/// SELECT lexical_position_after('n');  -- Returns a position after 'n'
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
/// SELECT lexical_position_before('n');  -- Returns a position before 'n'
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
    
    // Try to increment the last character
    if let Some(&last_char) = chars.last() {
        if last_char < END_CHAR {
            // We can increment the last character
            let mid_u8 = (last_char as u8 + END_CHAR as u8) / 2;
            if let Some(mid) = char::from_u32(mid_u8 as u32) {
                if mid > last_char {
                    let mut result: String = chars[..chars.len() - 1].iter().collect();
                    result.push(mid);
                    return result;
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
    
    // Try to decrement the last character
    if let Some(&last_char) = chars.last() {
        if last_char > START_CHAR {
            // We can decrement the last character
            let mid_u8 = (last_char as u8 + START_CHAR as u8) / 2;
            if let Some(mid) = char::from_u32(mid_u8 as u32) {
                if mid < last_char {
                    let mut result: String = chars[..chars.len() - 1].iter().collect();
                    result.push(mid);
                    return result;
                }
            }
        }
    }
    
    // If we can't decrement, prepend 'a' and add middle suffix
    // For a string like "a", we return "an" (which is less than "a" in some lexicographic schemes,
    // but for proper ordering we need to use a different approach)
    // Better approach: extend with a character before the last one
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
        
        if b_char < a_char {
            // Found a position where we can insert a character between
            let mid_u8 = (b_char as u8 + a_char as u8) / 2;
            if let Some(mid) = char::from_u32(mid_u8 as u32) {
                if mid > b_char && mid < a_char {
                    result.push(mid);
                    return result;
                } else if mid > b_char {
                    // mid == a_char, need to extend
                    result.push(b_char);
                    // Continue to find a spot
                } else {
                    // mid == b_char, push b_char and extend
                    result.push(b_char);
                }
            } else {
                // Fallback if char conversion fails
                result.push(b_char);
            }
        } else {
            result.push(b_char);
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
        assert_eq!(pos, "n");
    }

    #[pg_test]
    fn test_lexical_position_between_null_null() {
        let pos = crate::lexical_position_between(None, None);
        assert_eq!(pos, "n");
    }

    #[pg_test]
    fn test_lexical_position_after() {
        let pos = crate::lexical_position_after("n");
        assert!(pos > "n".to_string());
    }

    #[pg_test]
    fn test_lexical_position_before() {
        let pos = crate::lexical_position_before("n");
        assert!(pos < "n".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_with_before() {
        let pos = crate::lexical_position_between(Some("a"), None);
        assert!(pos > "a".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_with_after() {
        let pos = crate::lexical_position_between(None, Some("z"));
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_lexical_position_between_two_values() {
        let pos = crate::lexical_position_between(Some("a"), Some("z"));
        assert!(pos > "a".to_string());
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
        assert_eq!(lexical_position_first(), "n");
    }

    #[test]
    fn test_generate_after_basic() {
        let pos = generate_after("n");
        assert!(pos > "n".to_string());
    }

    #[test]
    fn test_generate_before_basic() {
        let pos = generate_before("n");
        assert!(pos < "n".to_string());
    }

    #[test]
    fn test_generate_between_basic() {
        let pos = generate_between("a", "z");
        assert!(pos > "a".to_string());
        assert!(pos < "z".to_string());
    }

    #[test]
    fn test_generate_between_adjacent() {
        // When characters are adjacent (like 'a' and 'b'), we need to extend
        let pos = generate_between("a", "b");
        assert!(pos > "a".to_string());
        assert!(pos < "b".to_string());
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
        assert_eq!(lexical_position_between(None, None), "n");
        
        let after_n = lexical_position_between(Some("n"), None);
        assert!(after_n > "n".to_string());
        
        let before_n = lexical_position_between(None, Some("n"));
        assert!(before_n < "n".to_string());
        
        let between = lexical_position_between(Some("a"), Some("z"));
        assert!(between > "a".to_string());
        assert!(between < "z".to_string());
    }

    // Edge case tests
    #[test]
    fn test_generate_after_empty_string() {
        let pos = generate_after("");
        assert_eq!(pos, "n");
    }

    #[test]
    fn test_generate_before_empty_string() {
        let pos = generate_before("");
        assert_eq!(pos, "n");
    }

    #[test]
    fn test_generate_between_empty_strings() {
        let pos = generate_between("", "");
        assert_eq!(pos, "n");
    }

    #[test]
    fn test_generate_between_before_empty() {
        let pos = generate_between("", "z");
        assert!(pos < "z".to_string());
    }

    #[test]
    fn test_generate_between_after_empty() {
        let pos = generate_between("a", "");
        assert!(pos > "a".to_string());
    }

    #[test]
    fn test_generate_between_invalid_order() {
        // When before >= after, should return position after 'before'
        let pos = generate_between("z", "a");
        assert!(pos > "z".to_string());
    }

    #[test]
    fn test_generate_between_equal_strings() {
        // When before == after, should return position after 'before'
        let pos = generate_between("n", "n");
        assert!(pos > "n".to_string());
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
