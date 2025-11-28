use pgrx::prelude::*;
use pgrx::{opname, pg_operator, StringInfo};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::CStr;
use std::hash::{Hash, Hasher};

::pgrx::pg_module_magic!();

/// A lexicographic ordering type for PostgreSQL.
///
/// The `Lexo` type provides lexicographically sortable position strings that can be used
/// for efficient ordering of items in database tables. It supports insertion of new items
/// between any two existing positions without requiring updates to other rows.
///
/// # Example
/// ```sql
/// CREATE TABLE items (
///     id SERIAL PRIMARY KEY,
///     position lexo.Lexo NOT NULL
/// );
///
/// -- Insert first item
/// INSERT INTO items (position) VALUES (lexo.new());
///
/// -- Query ordered items
/// SELECT * FROM items ORDER BY position;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct Lexo {
    value: String,
}

impl Lexo {
    /// Creates a new Lexo from a string value.
    ///
    /// # Arguments
    /// * `value` - The lexicographic position string
    #[inline]
    pub fn new(value: String) -> Self {
        Self { value }
    }

    /// Returns a reference to the inner string value.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Consumes the Lexo and returns the inner string value.
    #[inline]
    pub fn into_inner(self) -> String {
        self.value
    }
}

impl Default for Lexo {
    fn default() -> Self {
        Self { value: MID_CHAR.to_string() }
    }
}

impl From<String> for Lexo {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Lexo {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl AsRef<str> for Lexo {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for Lexo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl PartialEq for Lexo {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
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
        self.value.cmp(&other.value)
    }
}

impl Hash for Lexo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl InOutFuncs for Lexo {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        let s = input.to_str().expect("invalid UTF-8 in Lexo input");
        Lexo::new(s.to_string())
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&self.value);
    }
}

// Comparison operators for Lexo type to enable ORDER BY and comparisons
#[pg_operator(immutable, parallel_safe)]
#[opname(=)]
fn lexo_eq(left: Lexo, right: Lexo) -> bool {
    left == right
}

#[pg_operator(immutable, parallel_safe)]
#[opname(<>)]
fn lexo_ne(left: Lexo, right: Lexo) -> bool {
    left != right
}

#[pg_operator(immutable, parallel_safe)]
#[opname(<)]
fn lexo_lt(left: Lexo, right: Lexo) -> bool {
    left < right
}

#[pg_operator(immutable, parallel_safe)]
#[opname(<=)]
fn lexo_le(left: Lexo, right: Lexo) -> bool {
    left <= right
}

#[pg_operator(immutable, parallel_safe)]
#[opname(>)]
fn lexo_gt(left: Lexo, right: Lexo) -> bool {
    left > right
}

#[pg_operator(immutable, parallel_safe)]
#[opname(>=)]
fn lexo_ge(left: Lexo, right: Lexo) -> bool {
    left >= right
}

/// B-tree comparison function for Lexo type
#[pg_extern(immutable, parallel_safe)]
fn lexo_cmp(left: Lexo, right: Lexo) -> i32 {
    match left.cmp(&right) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// Hash function for Lexo type
#[pg_extern(immutable, parallel_safe)]
fn lexo_hash(value: Lexo) -> i32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish() as i32
}

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
    use crate::{generate_after, generate_before, generate_between, Lexo, MID_CHAR};

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

    // ============================================================================
    // Lexo type functions - Return the Lexo custom type
    // ============================================================================

    /// Creates a new Lexo position value.
    /// Returns the initial position (middle of base62: 'V') as a Lexo type.
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.new();  -- Returns 'V' as Lexo type
    /// ```
    #[pg_extern(name = "new", immutable, parallel_safe)]
    pub fn lexo_new() -> Lexo {
        Lexo::default()
    }

    /// Generates a Lexo position after the given position.
    ///
    /// # Arguments
    /// * `current` - The current Lexo position
    ///
    /// # Returns
    /// A new Lexo position that comes after `current`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.next('V'::lexo);  -- Returns a Lexo position after 'V'
    /// ```
    #[pg_extern(name = "next", immutable, parallel_safe)]
    pub fn lexo_next(current: Lexo) -> Lexo {
        Lexo::new(generate_after(current.as_str()))
    }

    /// Generates a Lexo position before the given position.
    ///
    /// # Arguments
    /// * `current` - The current Lexo position
    ///
    /// # Returns
    /// A new Lexo position that comes before `current`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.prev('V'::lexo);  -- Returns a Lexo position before 'V'
    /// ```
    #[pg_extern(name = "prev", immutable, parallel_safe)]
    pub fn lexo_prev(current: Lexo) -> Lexo {
        Lexo::new(generate_before(current.as_str()))
    }

    /// Generates a Lexo position between two positions.
    ///
    /// # Arguments
    /// * `before` - The Lexo position before the new position (can be NULL)
    /// * `after` - The Lexo position after the new position (can be NULL)
    ///
    /// # Returns
    /// A new Lexo position that falls between `before` and `after`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.mid(NULL::lexo, NULL::lexo);  -- Returns initial position 'V'
    /// SELECT lexo.mid('A'::lexo, 'Z'::lexo);    -- Returns midpoint between 'A' and 'Z'
    /// ```
    #[pg_extern(name = "mid", immutable, parallel_safe)]
    pub fn lexo_mid(before: Option<Lexo>, after: Option<Lexo>) -> Lexo {
        let before_str = before.as_ref().map(|l| l.as_str());
        let after_str = after.as_ref().map(|l| l.as_str());
        
        let result = match (before_str, after_str) {
            (None, None) => MID_CHAR.to_string(),
            (Some(b), None) => generate_after(b),
            (None, Some(a)) => generate_before(a),
            (Some(b), Some(a)) => generate_between(b, a),
        };
        
        Lexo::new(result)
    }

    /// Converts a TEXT value to a Lexo type.
    ///
    /// # Arguments
    /// * `value` - The text value to convert
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.from_text('V');  -- Returns 'V' as Lexo type
    /// ```
    #[pg_extern(name = "from_text", immutable, parallel_safe)]
    pub fn lexo_from_text(value: &str) -> Lexo {
        Lexo::new(value.to_string())
    }

    /// Converts a Lexo type to TEXT.
    ///
    /// # Arguments
    /// * `value` - The Lexo value to convert
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.to_text('V'::lexo);  -- Returns 'V' as TEXT
    /// ```
    #[pg_extern(name = "to_text", immutable, parallel_safe)]
    pub fn lexo_to_text(value: Lexo) -> String {
        value.into_inner()
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
    use crate::Lexo;

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

    // ============================================================================
    // Lexo type PostgreSQL integration tests
    // ============================================================================

    #[pg_test]
    fn test_lexo_type_new() {
        let pos = crate::lexo::lexo_new();
        assert_eq!(pos.as_str(), "V");
    }

    #[pg_test]
    fn test_lexo_type_next() {
        let first = crate::lexo::lexo_new();
        let second = crate::lexo::lexo_next(first.clone());
        assert!(second > first);
    }

    #[pg_test]
    fn test_lexo_type_prev() {
        let first = crate::lexo::lexo_new();
        let before = crate::lexo::lexo_prev(first.clone());
        assert!(before < first);
    }

    #[pg_test]
    fn test_lexo_type_mid() {
        let a = Lexo::from("A");
        let z = Lexo::from("Z");
        let mid = crate::lexo::lexo_mid(Some(a.clone()), Some(z.clone()));
        assert!(mid > a);
        assert!(mid < z);
    }

    #[pg_test]
    fn test_lexo_type_from_text() {
        let text = "ABC";
        let lexo = crate::lexo::lexo_from_text(text);
        assert_eq!(lexo.as_str(), "ABC");
    }

    #[pg_test]
    fn test_lexo_type_to_text() {
        let lexo = Lexo::from("ABC");
        let text = crate::lexo::lexo_to_text(lexo);
        assert_eq!(text, "ABC");
    }

    #[pg_test]
    fn test_lexo_operators() {
        let a = Lexo::from("A");
        let b = Lexo::from("B");
        let a2 = Lexo::from("A");
        
        // Test equality operators
        assert!(crate::lexo_eq(a.clone(), a2.clone()));
        assert!(!crate::lexo_ne(a.clone(), a2.clone()));
        
        // Test comparison operators
        assert!(crate::lexo_lt(a.clone(), b.clone()));
        assert!(crate::lexo_le(a.clone(), b.clone()));
        assert!(crate::lexo_le(a.clone(), a2.clone()));
        assert!(crate::lexo_gt(b.clone(), a.clone()));
        assert!(crate::lexo_ge(b.clone(), a.clone()));
        assert!(crate::lexo_ge(a.clone(), a2.clone()));
    }

    #[pg_test]
    fn test_lexo_cmp() {
        let a = Lexo::from("A");
        let b = Lexo::from("B");
        let a2 = Lexo::from("A");
        
        assert_eq!(crate::lexo_cmp(a.clone(), b.clone()), -1);
        assert_eq!(crate::lexo_cmp(b.clone(), a.clone()), 1);
        assert_eq!(crate::lexo_cmp(a.clone(), a2.clone()), 0);
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

    // ============================================================================
    // Lexo type tests
    // ============================================================================

    #[test]
    fn test_lexo_type_new() {
        let lexo = Lexo::new("V".to_string());
        assert_eq!(lexo.as_str(), "V");
    }

    #[test]
    fn test_lexo_type_default() {
        let lexo = Lexo::default();
        assert_eq!(lexo.as_str(), "V");
    }

    #[test]
    fn test_lexo_type_from_string() {
        let lexo: Lexo = "ABC".to_string().into();
        assert_eq!(lexo.as_str(), "ABC");
    }

    #[test]
    fn test_lexo_type_from_str() {
        let lexo: Lexo = "ABC".into();
        assert_eq!(lexo.as_str(), "ABC");
    }

    #[test]
    fn test_lexo_type_into_inner() {
        let lexo = Lexo::new("V".to_string());
        let inner: String = lexo.into_inner();
        assert_eq!(inner, "V");
    }

    #[test]
    fn test_lexo_type_display() {
        let lexo = Lexo::new("V".to_string());
        assert_eq!(format!("{}", lexo), "V");
    }

    #[test]
    fn test_lexo_type_equality() {
        let a = Lexo::new("V".to_string());
        let b = Lexo::new("V".to_string());
        let c = Lexo::new("A".to_string());
        
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_lexo_type_ordering() {
        let a = Lexo::new("A".to_string());
        let b = Lexo::new("V".to_string());
        let c = Lexo::new("z".to_string());
        
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
        assert!(c > a);
    }

    #[test]
    fn test_lexo_type_clone() {
        let original = Lexo::new("V".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_lexo_type_hash() {
        use std::collections::HashSet;
        
        let mut set = HashSet::new();
        set.insert(Lexo::new("A".to_string()));
        set.insert(Lexo::new("B".to_string()));
        set.insert(Lexo::new("A".to_string())); // duplicate
        
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_lexo_schema_functions_with_type() {
        // Test the new Lexo type functions
        let first = lexo::lexo_new();
        assert_eq!(first.as_str(), "V");
        
        let second = lexo::lexo_next(first.clone());
        assert!(second > first);
        
        let before_first = lexo::lexo_prev(first.clone());
        assert!(before_first < first);
    }

    #[test]
    fn test_lexo_mid_function() {
        // Test lexo_mid with different inputs
        let mid1 = lexo::lexo_mid(None, None);
        assert_eq!(mid1.as_str(), "V");
        
        let a = Lexo::new("A".to_string());
        let z = Lexo::new("Z".to_string());
        
        let between = lexo::lexo_mid(Some(a.clone()), Some(z.clone()));
        assert!(between > a);
        assert!(between < z);
    }

    #[test]
    fn test_lexo_conversion_functions() {
        let text = "ABC";
        let lexo = lexo::lexo_from_text(text);
        assert_eq!(lexo.as_str(), "ABC");
        
        let back_to_text = lexo::lexo_to_text(lexo);
        assert_eq!(back_to_text, text);
    }

    #[test]
    fn test_lexo_type_sequence() {
        // Test creating a sequence of Lexo values
        let first = lexo::lexo_new();
        let second = lexo::lexo_next(first.clone());
        let third = lexo::lexo_next(second.clone());
        
        assert!(first < second);
        assert!(second < third);
        
        // Insert between first and second
        let between = lexo::lexo_mid(Some(first.clone()), Some(second.clone()));
        assert!(first < between);
        assert!(between < second);
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
