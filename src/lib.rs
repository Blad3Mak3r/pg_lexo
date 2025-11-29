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

/// The lexo schema contains all public functions for lexicographic ordering
#[pg_schema]
mod lexo {
    use pgrx::prelude::*;
    use pgrx::spi::{Spi, quote_identifier};
    use crate::{generate_after, generate_before, generate_between, MID_CHAR};

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
    /// SELECT lexo.between(NULL, NULL);  -- Returns initial position 'V'
    /// SELECT lexo.between('V', NULL);   -- Returns position after 'V'
    /// SELECT lexo.between(NULL, 'V');   -- Returns position before 'V'
    /// SELECT lexo.between('A', 'Z');    -- Returns midpoint between 'A' and 'Z'
    /// ```
    #[pg_extern]
    pub fn between(before: Option<&str>, after: Option<&str>) -> String {
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
    /// SELECT lexo.first();  -- Returns 'V'
    /// ```
    #[pg_extern]
    pub fn first() -> String {
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
    /// SELECT lexo.after('V');  -- Returns a position after 'V'
    /// ```
    #[pg_extern]
    pub fn after(current: &str) -> String {
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
    /// SELECT lexo.before('V');  -- Returns a position before 'V'
    /// ```
    #[pg_extern]
    pub fn before(current: &str) -> String {
        generate_before(current)
    }

    /// Generates the next lexicographic position for a table column.
    /// 
    /// This function queries the specified table to find the maximum position value
    /// in the given column, then returns a position that comes after it. If the table
    /// is empty or the column contains only NULL values, it returns the initial position.
    /// 
    /// # Arguments
    /// * `table_name` - The name of the table (can be schema-qualified, e.g., 'public.my_table')
    /// * `column_name` - The name of the column containing position values
    /// 
    /// # Returns
    /// A new position string that comes after the maximum existing position,
    /// or the initial position if the table is empty.
    /// 
    /// # Example
    /// ```sql
    /// -- Get the next position for the 'position' column in 'items' table
    /// INSERT INTO items (id, position) VALUES (1, lexo.next_on_table('items', 'position'));
    /// 
    /// -- With schema-qualified table name
    /// SELECT lexo.next_on_table('public.items', 'position');
    /// ```
    #[pg_extern]
    pub fn next_on_table(table_name: &str, column_name: &str) -> String {
        // Safely quote the identifiers to prevent SQL injection
        let quoted_column = quote_identifier(column_name);
        
        // Handle schema-qualified table names (e.g., 'public.my_table')
        // by quoting each part separately
        let quoted_table = if table_name.contains('.') {
            table_name
                .split('.')
                .map(quote_identifier)
                .collect::<Vec<_>>()
                .join(".")
        } else {
            quote_identifier(table_name)
        };
        
        // Build the query to find the maximum position
        let query = format!(
            "SELECT MAX({}) FROM {}",
            quoted_column, quoted_table
        );
        
        // Execute the query and get the maximum position
        // If the query fails (e.g., table doesn't exist), we propagate the error
        let max_position: Option<String> = Spi::get_one(&query)
            .expect("Failed to query table for maximum position");
        
        match max_position {
            Some(pos) => generate_after(&pos),
            None => MID_CHAR.to_string(),
        }
    }
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
    fn test_lexo_first() {
        let pos = crate::lexo::first();
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_lexo_between_null_null() {
        let pos = crate::lexo::between(None, None);
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_lexo_after() {
        let pos = crate::lexo::after("V");
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_lexo_before() {
        let pos = crate::lexo::before("V");
        assert!(pos < "V".to_string());
    }

    #[pg_test]
    fn test_lexo_between_with_before() {
        let pos = crate::lexo::between(Some("0"), None);
        assert!(pos > "0".to_string());
    }

    #[pg_test]
    fn test_lexo_between_with_after() {
        let pos = crate::lexo::between(None, Some("z"));
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_lexo_between_two_values() {
        let pos = crate::lexo::between(Some("0"), Some("z"));
        assert!(pos > "0".to_string());
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_ordering_sequence() {
        // Test that we can create a sequence of ordered positions
        let first = crate::lexo::first();
        let second = crate::lexo::after(&first);
        let third = crate::lexo::after(&second);
        
        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_insert_between_sequence() {
        // Test inserting between existing positions
        let first = crate::lexo::first();
        let third = crate::lexo::after(&first);
        let second = crate::lexo::between(Some(&first), Some(&third));
        
        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_next_on_table_empty() {
        use pgrx::spi::Spi;
        
        // Create a test table
        Spi::run("CREATE TEMPORARY TABLE test_empty (id SERIAL PRIMARY KEY, position TEXT)").unwrap();
        
        // Get next position on empty table - should return first position
        let pos = crate::lexo::next_on_table("test_empty", "position");
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_next_on_table_with_data() {
        use pgrx::spi::Spi;
        
        // Create a test table with some data
        Spi::run("CREATE TEMPORARY TABLE test_data (id SERIAL PRIMARY KEY, position TEXT)").unwrap();
        Spi::run("INSERT INTO test_data (position) VALUES ('V')").unwrap();
        
        // Get next position - should be after 'V'
        let pos = crate::lexo::next_on_table("test_data", "position");
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_next_on_table_multiple_rows() {
        use pgrx::spi::Spi;
        
        // Create a test table with multiple rows
        Spi::run("CREATE TEMPORARY TABLE test_multi (id SERIAL PRIMARY KEY, position TEXT)").unwrap();
        Spi::run("INSERT INTO test_multi (position) VALUES ('A'), ('M'), ('Z')").unwrap();
        
        // Get next position - should be after 'Z' (the max)
        let pos = crate::lexo::next_on_table("test_multi", "position");
        assert!(pos > "Z".to_string());
    }

    #[pg_test]
    fn test_next_on_table_with_nulls() {
        use pgrx::spi::Spi;
        
        // Create a test table with NULL values
        Spi::run("CREATE TEMPORARY TABLE test_nulls (id SERIAL PRIMARY KEY, position TEXT)").unwrap();
        Spi::run("INSERT INTO test_nulls (position) VALUES (NULL), ('V'), (NULL)").unwrap();
        
        // Get next position - should be after 'V' (NULL values are ignored by MAX)
        let pos = crate::lexo::next_on_table("test_nulls", "position");
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_next_on_table_only_nulls() {
        use pgrx::spi::Spi;
        
        // Create a test table with only NULL values
        Spi::run("CREATE TEMPORARY TABLE test_only_nulls (id SERIAL PRIMARY KEY, position TEXT)").unwrap();
        Spi::run("INSERT INTO test_only_nulls (position) VALUES (NULL), (NULL)").unwrap();
        
        // Get next position - should return first position since all are NULL
        let pos = crate::lexo::next_on_table("test_only_nulls", "position");
        assert_eq!(pos, "V");
    }
}

/// Standard Rust unit tests that don't require PostgreSQL
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_generate_first() {
        assert_eq!(lexo::first(), "V");
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
        let first = lexo::first();
        let second = generate_after(&first);
        let third = generate_after(&second);
        let fourth = generate_after(&third);
        
        assert!(first < second);
        assert!(second < third);
        assert!(third < fourth);
    }

    #[test]
    fn test_insert_between_maintains_order() {
        let first = lexo::first();
        let third = generate_after(&first);
        let second = generate_between(&first, &third);
        
        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn test_multiple_insertions() {
        // Simulate multiple insertions
        let mut positions = vec![lexo::first()];
        
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
        let first = lexo::first();
        let before_first = generate_before(&first);
        
        assert!(before_first < first);
    }

    #[test]
    fn test_lexo_between_function() {
        // Test the public API
        assert_eq!(lexo::between(None, None), "V");
        
        let after_v = lexo::between(Some("V"), None);
        assert!(after_v > "V".to_string());
        
        let before_v = lexo::between(None, Some("V"));
        assert!(before_v < "V".to_string());
        
        let between = lexo::between(Some("0"), Some("z"));
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
