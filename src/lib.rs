use pgrx::prelude::*;
use pgrx::spi::{Spi, quote_identifier, quote_literal};

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

/// Check if a string contains only valid Base62 characters
fn is_valid_base62(s: &str) -> bool {
    s.chars().all(|c| BASE62_CHARS.contains(&(c as u8)))
}

/// Get the index of a character in the base62 character set
fn char_to_index(c: char) -> Option<usize> {
    BASE62_CHARS.iter().position(|&x| x == c as u8)
}

/// Get the character at a given index in the base62 character set
fn index_to_char(idx: usize) -> Option<char> {
    BASE62_CHARS.get(idx).map(|&b| b as char)
}

/// The `lexo` type is installed in `pg_catalog` schema so it's globally available
/// without needing to qualify the schema name, similar to how PostgreSQL's built-in
/// types like `uuid` work.
#[pg_schema]
pub mod pg_catalog {
    use pgrx::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;
    use std::fmt;
    use std::str::FromStr;
    use super::is_valid_base62;

    /// Custom lexo type for lexicographic ordering positions.
    /// 
    /// This type provides:
    /// - Built-in byte-order comparison (equivalent to COLLATE "C")
    /// - Automatic validation of Base62 characters
    /// - Type safety (prevents mixing with regular text)
    /// - Installed in pg_catalog for global availability
    /// 
    /// # Example
    /// ```sql
    /// CREATE TABLE items (
    ///     id SERIAL PRIMARY KEY,
    ///     position lexo NOT NULL
    /// );
    /// 
    /// INSERT INTO items (position) VALUES (lexo_first());
    /// SELECT * FROM items ORDER BY position;  -- No COLLATE needed!
    /// ```
    #[derive(Debug, Clone, Serialize, Deserialize, PostgresType, PostgresEq, PostgresOrd, PostgresHash)]
    #[inoutfuncs]
    pub struct Lexo(pub(crate) String);

    impl Lexo {
        /// Create a new Lexo from a string, validating that it contains only Base62 characters
        pub fn new(s: &str) -> Result<Self, String> {
            if s.is_empty() {
                return Err("Lexo position cannot be empty".to_string());
            }
            if !is_valid_base62(s) {
                return Err(format!(
                    "Invalid lexo position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                    s
                ));
            }
            Ok(Lexo(s.to_string()))
        }

        /// Create a new Lexo without validation (internal use only)
        pub(crate) fn new_unchecked(s: String) -> Self {
            Lexo(s)
        }

        /// Get the inner string value
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl fmt::Display for Lexo {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl FromStr for Lexo {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Lexo::new(s)
        }
    }

    impl InOutFuncs for Lexo {
        fn input(input: &core::ffi::CStr) -> Self
        where
            Self: Sized,
        {
            let s = input.to_str().expect("Invalid UTF-8 in lexo input");
            match Lexo::new(s) {
                Ok(lexo) => lexo,
                Err(e) => pgrx::error!("{}", e),
            }
        }

        fn output(&self, buffer: &mut pgrx::StringInfo) {
            buffer.push_str(&self.0);
        }
    }

    // Implement ordering using byte comparison (equivalent to COLLATE "C")
    impl PartialEq for Lexo {
        fn eq(&self, other: &Self) -> bool {
            self.0.as_bytes() == other.0.as_bytes()
        }
    }

    impl Eq for Lexo {}

    impl PartialOrd for Lexo {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Lexo {
        fn cmp(&self, other: &Self) -> Ordering {
            // Use byte comparison for C collation semantics
            self.0.as_bytes().cmp(other.0.as_bytes())
        }
    }

    impl std::hash::Hash for Lexo {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }
}

// Re-export Lexo for use in the rest of the crate
pub use pg_catalog::Lexo;

/// Generates a lexicographic position string that comes between two positions.
/// 
/// This function is used to insert items between existing positions in an ordered list.
/// 
/// # Arguments
/// * `before` - The position before the new position (can be NULL for first position)
/// * `after` - The position after the new position (can be NULL for last position)
/// 
/// # Returns
/// A new lexo position that lexicographically falls between `before` and `after`
/// 
/// # Example
/// ```sql
/// SELECT lexo_between(NULL, NULL);  -- Returns initial position 'V'
/// SELECT lexo_between('V'::lexo, NULL);   -- Returns position after 'V'
/// SELECT lexo_between(NULL, 'V'::lexo);   -- Returns position before 'V'
/// SELECT lexo_between('A'::lexo, 'Z'::lexo);    -- Returns midpoint between 'A' and 'Z'
/// ```
#[pg_extern]
pub fn lexo_between(before: Option<Lexo>, after: Option<Lexo>) -> Lexo {
    let result = match (&before, &after) {
        (None, None) => {
            // First position in an empty list
            MID_CHAR.to_string()
        }
        (Some(b), None) => {
            // Position after the last item
            generate_after(b.as_str())
        }
        (None, Some(a)) => {
            // Position before the first item
            generate_before(a.as_str())
        }
        (Some(b), Some(a)) => {
            // Position between two existing items
            generate_between(b.as_str(), a.as_str())
        }
    };
    Lexo::new_unchecked(result)
}

/// Generates the first lexicographic position for a new ordered list.
/// 
/// # Returns
/// The initial lexo position (middle of base62: 'V')
/// 
/// # Example
/// ```sql
/// SELECT lexo_first();  -- Returns 'V'
/// ```
#[pg_extern]
pub fn lexo_first() -> Lexo {
    Lexo::new_unchecked(MID_CHAR.to_string())
}

/// Generates a lexicographic position after the given position.
/// 
/// # Arguments
/// * `current` - The current position
/// 
/// # Returns
/// A new lexo position that comes after `current`
/// 
/// # Example
/// ```sql
/// SELECT lexo_after('V'::lexo);  -- Returns a position after 'V'
/// ```
#[pg_extern]
pub fn lexo_after(current: Lexo) -> Lexo {
    Lexo::new_unchecked(generate_after(current.as_str()))
}

/// Generates a lexicographic position before the given position.
/// 
/// # Arguments
/// * `current` - The current position
/// 
/// # Returns
/// A new lexo position that comes before `current`
/// 
/// # Example
/// ```sql
/// SELECT lexo_before('V'::lexo);  -- Returns a position before 'V'
/// ```
#[pg_extern]
pub fn lexo_before(current: Lexo) -> Lexo {
    Lexo::new_unchecked(generate_before(current.as_str()))
}

/// Generates the next lexicographic position for a table column (after the last/maximum position).
/// 
/// This function queries the specified table to find the maximum position value
/// in the given column, then returns a position that comes after it. If the table
/// is empty or the column contains only NULL values, it returns the initial position.
/// 
/// # Arguments
/// * `table_name` - The name of the table (can be schema-qualified, e.g., 'public.my_table')
/// * `lexo_column_name` - The name of the column containing lexo position values
/// * `identifier_column_name` - Optional name of the column to filter by (e.g., 'collection_id')
/// * `identifier_value` - Optional value to filter by (e.g., the actual collection UUID)
/// 
/// # Returns
/// A new lexo position that comes after the maximum existing position,
/// or the initial position if no matching rows exist.
/// 
/// # Example
/// ```sql
/// -- Get the next position for the 'position' column in 'items' table (no filter)
/// INSERT INTO items (id, position) VALUES (1, lexo_next('items', 'position', NULL, NULL));
/// 
/// -- Get the next position for a specific collection in a relationship table
/// -- This finds MAX(position) WHERE collection_id = '832498y234-234wa'
/// INSERT INTO collection_songs (collection_id, song_id, position) 
/// VALUES ('832498y234-234wa', 'song123', lexo_next('collection_songs', 'position', 'collection_id', '832498y234-234wa'));
/// 
/// -- With schema-qualified table name
/// SELECT lexo_next('public.items', 'position', NULL, NULL);
/// ```
#[pg_extern]
pub fn lexo_next(
    table_name: &str, 
    lexo_column_name: &str, 
    identifier_column_name: Option<&str>,
    identifier_value: Option<&str>
) -> Lexo {
    // Safely quote the identifiers to prevent SQL injection
    let quoted_lexo_column = quote_identifier(lexo_column_name);
    
    // Handle schema-qualified table names (e.g., 'public.my_table')
    // by quoting each part separately
    let quoted_table = if let Some((schema, table)) = table_name.split_once('.') {
        format!("{}.{}", quote_identifier(schema), quote_identifier(table))
    } else {
        quote_identifier(table_name)
    };
    
    // Build the query based on whether we have filter parameters
    let query = match (identifier_column_name, identifier_value) {
        (Some(id_col), Some(id_val)) => {
            let quoted_id_column = quote_identifier(id_col);
            let quoted_id_value = quote_literal(id_val);
            format!(
                "SELECT MAX({}::text) FROM {} WHERE {} = {}",
                quoted_lexo_column, quoted_table, quoted_id_column, quoted_id_value
            )
        }
        _ => {
            // No filter, just get the max from the entire table
            format!(
                "SELECT MAX({}::text) FROM {}",
                quoted_lexo_column, quoted_table
            )
        }
    };
    
    // Execute the query and get the maximum position
    // If the query fails (e.g., table doesn't exist), we propagate the error
    let max_position: Option<String> = Spi::get_one(&query)
        .expect("Failed to query table for maximum position");
    
    match max_position {
        Some(pos) => Lexo::new_unchecked(generate_after(&pos)),
        None => Lexo::new_unchecked(MID_CHAR.to_string()),
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
        
        let b_idx = char_to_index(b_char)
            .expect("Invalid base62 character in before string");
        let a_idx = char_to_index(a_char)
            .expect("Invalid base62 character in after string");
        
        if b_idx == a_idx {
            // Characters are equal, add to result and continue
            result.push(b_char);
        } else if b_idx < a_idx {
            // Found a gap, try to find midpoint
            let mid_idx = (b_idx + a_idx) / 2;
            
            if mid_idx > b_idx {
                // We can insert at the midpoint
                if let Some(mid) = index_to_char(mid_idx) {
                    result.push(mid);
                    return result;
                }
            }
            
            // Adjacent characters, need to extend
            result.push(b_char);
            result.push(MID_CHAR);
            return result;
        } else {
            // b_idx > a_idx can happen when strings have different lengths
            // e.g., before="AB" (B=11) vs after="A" (implicit END_CHAR=61 at position 1)
            // In this case, we preserve b_char and continue looking for a gap
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
    use crate::Lexo;

    #[pg_test]
    fn test_lexo_first() {
        let pos = crate::lexo_first();
        assert_eq!(pos.as_str(), "V");
    }

    #[pg_test]
    fn test_lexo_between_null_null() {
        let pos = crate::lexo_between(None, None);
        assert_eq!(pos.as_str(), "V");
    }

    #[pg_test]
    fn test_lexo_after() {
        let v = Lexo::new("V").unwrap();
        let pos = crate::lexo_after(v);
        assert!(pos.as_str() > "V");
    }

    #[pg_test]
    fn test_lexo_before() {
        let v = Lexo::new("V").unwrap();
        let pos = crate::lexo_before(v);
        assert!(pos.as_str() < "V");
    }

    #[pg_test]
    fn test_lexo_between_with_before() {
        let zero = Lexo::new("0").unwrap();
        let pos = crate::lexo_between(Some(zero), None);
        assert!(pos.as_str() > "0");
    }

    #[pg_test]
    fn test_lexo_between_with_after() {
        let z = Lexo::new("z").unwrap();
        let pos = crate::lexo_between(None, Some(z));
        assert!(pos.as_str() < "z");
    }

    #[pg_test]
    fn test_lexo_between_two_values() {
        let zero = Lexo::new("0").unwrap();
        let z = Lexo::new("z").unwrap();
        let pos = crate::lexo_between(Some(zero), Some(z));
        assert!(pos.as_str() > "0");
        assert!(pos.as_str() < "z");
    }

    #[pg_test]
    fn test_ordering_sequence() {
        // Test that we can create a sequence of ordered positions
        let first = crate::lexo_first();
        let second = crate::lexo_after(first.clone());
        let third = crate::lexo_after(second.clone());
        
        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_insert_between_sequence() {
        // Test inserting between existing positions
        let first = crate::lexo_first();
        let third = crate::lexo_after(first.clone());
        let second = crate::lexo_between(Some(first.clone()), Some(third.clone()));
        
        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_next_empty() {
        use pgrx::spi::Spi;
        
        // Create a test table with lexo type
        Spi::run("CREATE TEMPORARY TABLE test_empty (id SERIAL PRIMARY KEY, position lexo)").unwrap();
        
        // Get next position on empty table - should return first position
        let pos = crate::lexo_next("test_empty", "position", None, None);
        assert_eq!(pos.as_str(), "V");
    }

    #[pg_test]
    fn test_next_with_data() {
        use pgrx::spi::Spi;
        
        // Create a test table with lexo type
        Spi::run("CREATE TEMPORARY TABLE test_data (id SERIAL PRIMARY KEY, position lexo)").unwrap();
        Spi::run("INSERT INTO test_data (position) VALUES ('V')").unwrap();
        
        // Get next position - should be after 'V'
        let pos = crate::lexo_next("test_data", "position", None, None);
        assert!(pos.as_str() > "V");
    }

    #[pg_test]
    fn test_next_multiple_rows() {
        use pgrx::spi::Spi;
        
        // Create a test table with lexo type
        Spi::run("CREATE TEMPORARY TABLE test_multi (id SERIAL PRIMARY KEY, position lexo)").unwrap();
        Spi::run("INSERT INTO test_multi (position) VALUES ('A'), ('M'), ('Z')").unwrap();
        
        // Get next position - should be after 'Z' (the max)
        let pos = crate::lexo_next("test_multi", "position", None, None);
        assert!(pos.as_str() > "Z");
    }

    #[pg_test]
    fn test_next_with_nulls() {
        use pgrx::spi::Spi;
        
        // Create a test table with lexo type and NULL values
        Spi::run("CREATE TEMPORARY TABLE test_nulls (id SERIAL PRIMARY KEY, position lexo)").unwrap();
        Spi::run("INSERT INTO test_nulls (position) VALUES (NULL), ('V'), (NULL)").unwrap();
        
        // Get next position - should be after 'V' (NULL values are ignored by MAX)
        let pos = crate::lexo_next("test_nulls", "position", None, None);
        assert!(pos.as_str() > "V");
    }

    #[pg_test]
    fn test_next_only_nulls() {
        use pgrx::spi::Spi;
        
        // Create a test table with only NULL values
        Spi::run("CREATE TEMPORARY TABLE test_only_nulls (id SERIAL PRIMARY KEY, position lexo)").unwrap();
        Spi::run("INSERT INTO test_only_nulls (position) VALUES (NULL), (NULL)").unwrap();
        
        // Get next position - should return first position since all are NULL
        let pos = crate::lexo_next("test_only_nulls", "position", None, None);
        assert_eq!(pos.as_str(), "V");
    }

    #[pg_test]
    fn test_next_with_filter() {
        use pgrx::spi::Spi;
        
        // Create a test table simulating a relationship table (like collection_songs)
        Spi::run("CREATE TEMPORARY TABLE test_collection_songs (collection_id TEXT, song_id TEXT, position lexo, PRIMARY KEY (collection_id, song_id))").unwrap();
        Spi::run("INSERT INTO test_collection_songs (collection_id, song_id, position) VALUES ('col1', 'song1', 'A'), ('col1', 'song2', 'M'), ('col2', 'song3', 'Z')").unwrap();
        
        // Get next position for col1 - should be after 'M' (max for col1)
        let pos = crate::lexo_next("test_collection_songs", "position", Some("collection_id"), Some("col1"));
        assert!(pos.as_str() > "M");
        
        // Get next position for col2 - should be after 'Z'
        let pos2 = crate::lexo_next("test_collection_songs", "position", Some("collection_id"), Some("col2"));
        assert!(pos2.as_str() > "Z");
        
        // Get next position for non-existent collection - should return first position
        let pos3 = crate::lexo_next("test_collection_songs", "position", Some("collection_id"), Some("col3"));
        assert_eq!(pos3.as_str(), "V");
    }

    #[pg_test]
    fn test_lexo_type_ordering() {
        // Test that the lexo type sorts correctly
        let a = Lexo::new("A").unwrap();
        let z = Lexo::new("Z").unwrap();
        let lower_a = Lexo::new("a").unwrap();
        
        // Verify C collation ordering: A < Z < a
        assert!(a < z);
        assert!(z < lower_a);
    }

    #[pg_test]
    fn test_lexo_type_validation() {
        // Valid Base62 should work
        assert!(Lexo::new("V").is_ok());
        assert!(Lexo::new("abc123XYZ").is_ok());
        
        // Invalid characters should fail
        assert!(Lexo::new("hello!").is_err());
        assert!(Lexo::new("test-value").is_err());
        assert!(Lexo::new("").is_err());
    }
}

/// Standard Rust unit tests that don't require PostgreSQL
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_generate_first() {
        let first = lexo_first();
        assert_eq!(first.as_str(), "V");
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
        let first = lexo_first();
        let second = generate_after(first.as_str());
        let third = generate_after(&second);
        let fourth = generate_after(&third);
        
        assert!(first.as_str() < second.as_str());
        assert!(second < third);
        assert!(third < fourth);
    }

    #[test]
    fn test_insert_between_maintains_order() {
        let first = lexo_first();
        let third = generate_after(first.as_str());
        let second = generate_between(first.as_str(), &third);
        
        assert!(first.as_str() < second.as_str());
        assert!(second < third);
    }

    #[test]
    fn test_multiple_insertions() {
        // Simulate multiple insertions
        let first = lexo_first();
        let mut positions: Vec<String> = vec![first.as_str().to_string()];
        
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
        let first = lexo_first();
        let before_first = generate_before(first.as_str());
        
        assert!(before_first < first.as_str().to_string());
    }

    #[test]
    fn test_lexo_between_function() {
        // Test the public API
        let between_null = lexo_between(None, None);
        assert_eq!(between_null.as_str(), "V");
        
        let v = Lexo::new("V").unwrap();
        let after_v = lexo_between(Some(v.clone()), None);
        assert!(after_v.as_str() > "V");
        
        let before_v = lexo_between(None, Some(v));
        assert!(before_v.as_str() < "V");
        
        let zero = Lexo::new("0").unwrap();
        let z = Lexo::new("z").unwrap();
        let between = lexo_between(Some(zero), Some(z));
        assert!(between.as_str() > "0");
        assert!(between.as_str() < "z");
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

    #[test]
    fn test_generate_between_same_prefix() {
        let pos = generate_between("AB", "AC");
        assert!(pos > "AB".to_string());
        assert!(pos < "AC".to_string());
        assert!(pos.starts_with("AB"));
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
            assert!(mid > positions[0], "mid {} should be > {}", mid, positions[0]);
            assert!(mid < positions[1], "mid {} should be < {}", mid, positions[1]);
            positions.insert(1, mid);
        }
    }

    #[test]
    fn test_generate_between_different_lengths() {
        let pos = generate_between("z", "z0");
        assert!(pos > "z".to_string());
        assert!(pos < "z0".to_string());
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
