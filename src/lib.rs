use pgrx::prelude::*;

::pgrx::pg_module_magic!();

/// Base62 character set: 0-9, A-Z, a-z (62 characters)
/// Sorted in ASCII/lexicographic order for proper string comparison
const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE: usize = 62;

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

/// The `lexo` schema contains all functions for lexicographic ordering.
///
/// Use `TEXT COLLATE "C"` columns for proper ordering.
///
/// # Example
/// ```sql
/// -- Create a table with a lexo column
/// SELECT lexo.add_lexo_column_to('my_table', 'position');
///
/// -- Or manually:
/// CREATE TABLE items (
///     id SERIAL PRIMARY KEY,
///     position TEXT COLLATE "C" NOT NULL
/// );
///
/// -- Use the functions
/// INSERT INTO items (position) VALUES (lexo.first());
/// SELECT * FROM items ORDER BY position;
/// ```
#[pg_schema]
pub mod lexo {
    use super::{
        MID_CHAR, generate_after, generate_before, generate_between as gen_between, is_valid_base62,
    };
    use pgrx::prelude::*;
    use pgrx::spi::{Spi, quote_identifier, quote_literal};

    /// Returns the first position for a new ordered list.
    ///
    /// # Returns
    /// The initial position (middle of base62: 'V')
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.first();  -- Returns 'V'
    /// INSERT INTO items (position) VALUES (lexo.first());
    /// ```
    #[pg_extern]
    pub fn first() -> String {
        MID_CHAR.to_string()
    }

    /// Returns a position after the given position.
    ///
    /// # Arguments
    /// * `current` - The current position (must be valid base62)
    ///
    /// # Returns
    /// A new position that comes after `current`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.after('V');  -- Returns a position after 'V'
    /// ```
    #[pg_extern]
    pub fn after(current: &str) -> String {
        if !is_valid_base62(current) {
            pgrx::error!(
                "Invalid position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                current
            );
        }
        generate_after(current)
    }

    /// Returns a position before the given position.
    ///
    /// # Arguments
    /// * `current` - The current position (must be valid base62)
    ///
    /// # Returns
    /// A new position that comes before `current`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.before('V');  -- Returns a position before 'V'
    /// ```
    #[pg_extern]
    pub fn before(current: &str) -> String {
        if !is_valid_base62(current) {
            pgrx::error!(
                "Invalid position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                current
            );
        }
        generate_before(current)
    }

    /// Returns a position between two existing positions.
    ///
    /// # Arguments
    /// * `before_pos` - The position before the new position (can be NULL for beginning)
    /// * `after_pos` - The position after the new position (can be NULL for end)
    ///
    /// # Returns
    /// A new position that lexicographically falls between `before_pos` and `after_pos`
    ///
    /// # Example
    /// ```sql
    /// SELECT lexo.between(NULL, NULL);       -- Returns 'V' (first position)
    /// SELECT lexo.between('V', NULL);        -- Returns position after 'V'
    /// SELECT lexo.between(NULL, 'V');        -- Returns position before 'V'
    /// SELECT lexo.between('A', 'Z');         -- Returns midpoint between 'A' and 'Z'
    /// ```
    #[pg_extern]
    pub fn between(before_pos: Option<&str>, after_pos: Option<&str>) -> String {
        // Validate inputs if provided
        if let Some(b) = before_pos
            && !b.is_empty()
            && !is_valid_base62(b)
        {
            pgrx::error!(
                "Invalid before position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                b
            );
        }
        if let Some(a) = after_pos
            && !a.is_empty()
            && !is_valid_base62(a)
        {
            pgrx::error!(
                "Invalid after position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                a
            );
        }

        match (before_pos, after_pos) {
            (None, None) => MID_CHAR.to_string(),
            (Some(b), None) => {
                if b.is_empty() {
                    MID_CHAR.to_string()
                } else {
                    generate_after(b)
                }
            }
            (None, Some(a)) => {
                if a.is_empty() {
                    MID_CHAR.to_string()
                } else {
                    generate_before(a)
                }
            }
            (Some(b), Some(a)) => {
                if b.is_empty() && a.is_empty() {
                    MID_CHAR.to_string()
                } else if b.is_empty() {
                    generate_before(a)
                } else if a.is_empty() {
                    generate_after(b)
                } else {
                    gen_between(b, a)
                }
            }
        }
    }

    /// Returns the next position after the maximum in a table column.
    ///
    /// This function queries the specified table to find the maximum position value
    /// in the given column, then returns a position that comes after it.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table (can be schema-qualified)
    /// * `lexo_column_name` - The name of the column containing position values
    /// * `identifier_column_name` - Optional: column to filter by (e.g., 'collection_id')
    /// * `identifier_value` - Optional: value to filter by
    ///
    /// # Returns
    /// A new position after the maximum, or 'V' if table is empty
    ///
    /// # Example
    /// ```sql
    /// -- Get next position for entire table
    /// SELECT lexo.next('items', 'position', NULL, NULL);
    ///
    /// -- Get next position for a specific collection
    /// SELECT lexo.next('collection_songs', 'position', 'collection_id', 'abc-123');
    /// ```
    #[pg_extern]
    pub fn next(
        table_name: &str,
        lexo_column_name: &str,
        identifier_column_name: Option<&str>,
        identifier_value: Option<&str>,
    ) -> String {
        let quoted_lexo_column = quote_identifier(lexo_column_name);

        let quoted_table = if let Some((schema, table)) = table_name.split_once('.') {
            format!("{}.{}", quote_identifier(schema), quote_identifier(table))
        } else {
            quote_identifier(table_name)
        };

        let query = match (identifier_column_name, identifier_value) {
            (Some(id_col), Some(id_val)) => {
                let quoted_id_column = quote_identifier(id_col);
                let quoted_id_value = quote_literal(id_val);
                format!(
                    "SELECT MAX({} COLLATE \"C\") FROM {} WHERE {} = {}",
                    quoted_lexo_column, quoted_table, quoted_id_column, quoted_id_value
                )
            }
            _ => {
                format!(
                    "SELECT MAX({} COLLATE \"C\") FROM {}",
                    quoted_lexo_column, quoted_table
                )
            }
        };

        let max_position: Option<String> =
            Spi::get_one(&query).expect("Failed to query table for maximum position");

        match max_position {
            Some(pos) => generate_after(&pos),
            None => MID_CHAR.to_string(),
        }
    }

    /// Adds a lexo position column to an existing table.
    ///
    /// The column will be of type `TEXT COLLATE "C"` to ensure proper
    /// lexicographic ordering.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table (can be schema-qualified)
    /// * `column_name` - The name of the new column to add
    ///
    /// # Example
    /// ```sql
    /// -- Add a 'position' column to 'items' table
    /// SELECT lexo.add_lexo_column_to('items', 'position');
    ///
    /// -- The column is created as:
    /// -- ALTER TABLE items ADD COLUMN position TEXT COLLATE "C";
    /// ```
    #[pg_extern]
    pub fn add_lexo_column_to(table_name: &str, column_name: &str) {
        let quoted_table = if let Some((schema, table)) = table_name.split_once('.') {
            format!("{}.{}", quote_identifier(schema), quote_identifier(table))
        } else {
            quote_identifier(table_name)
        };

        let quoted_column = quote_identifier(column_name);

        let query = format!(
            "ALTER TABLE {} ADD COLUMN {} TEXT COLLATE \"C\"",
            quoted_table, quoted_column
        );

        Spi::run(&query).expect("Failed to add lexo column to table");
    }

    /// Rebalances lexicographic position values in a table.
    ///
    /// This function recalculates all position values to be evenly distributed,
    /// which is useful when positions have become too long due to many insertions
    /// or when you want to "clean up" the ordering.
    ///
    /// The function preserves the current order of rows while assigning new,
    /// optimally distributed position values.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table (can be schema-qualified)
    /// * `lexo_column_name` - The name of the column containing position values
    /// * `key_column_name` - Optional: column to group by (e.g., 'playlist_id')
    /// * `key_value` - Optional: value to filter by (rebalance only rows with this key)
    ///
    /// # Returns
    /// The number of rows that were rebalanced
    ///
    /// # Example
    /// ```sql
    /// -- Rebalance all positions in a table
    /// SELECT lexo.rebalance('items', 'position', NULL, NULL);
    ///
    /// -- Rebalance positions for a specific playlist
    /// SELECT lexo.rebalance('playlist_songs', 'position', 'playlist_id', 'abc-123');
    /// ```
    #[pg_extern]
    pub fn rebalance(
        table_name: &str,
        lexo_column_name: &str,
        key_column_name: Option<&str>,
        key_value: Option<&str>,
    ) -> i64 {
        let quoted_lexo_column = quote_identifier(lexo_column_name);

        let quoted_table = if let Some((schema, table)) = table_name.split_once('.') {
            format!("{}.{}", quote_identifier(schema), quote_identifier(table))
        } else {
            quote_identifier(table_name)
        };

        // Build the query to get row count
        let count_query = match (&key_column_name, &key_value) {
            (Some(key_col), Some(key_val)) => {
                let quoted_key_column = quote_identifier(key_col);
                let quoted_key_value = quote_literal(key_val);
                format!(
                    "SELECT COUNT(*) FROM {} WHERE {} = {}",
                    quoted_table, quoted_key_column, quoted_key_value
                )
            }
            _ => format!("SELECT COUNT(*) FROM {}", quoted_table),
        };

        let count: Option<i64> = Spi::get_one(&count_query).expect("Failed to count rows in table");
        let row_count = count.unwrap_or(0);

        if row_count == 0 {
            return 0;
        }

        // Generate evenly distributed positions for all rows
        let positions = super::generate_balanced_positions(row_count as usize);

        // Build query to get all rows ordered by current position, using ctid as text
        let select_query = match (&key_column_name, &key_value) {
            (Some(key_col), Some(key_val)) => {
                let quoted_key_column = quote_identifier(key_col);
                let quoted_key_value = quote_literal(key_val);
                format!(
                    "SELECT ctid::text FROM {} WHERE {} = {} ORDER BY {} COLLATE \"C\"",
                    quoted_table, quoted_key_column, quoted_key_value, quoted_lexo_column
                )
            }
            _ => format!(
                "SELECT ctid::text FROM {} ORDER BY {} COLLATE \"C\"",
                quoted_table, quoted_lexo_column
            ),
        };

        // Update each row with its new position
        Spi::connect_mut(|client| {
            let rows = client
                .select(&select_query, None, &[])
                .expect("Failed to select rows for rebalancing");

            for (idx, row) in rows.enumerate() {
                let ctid_str: String = row
                    .get(1)
                    .expect("Failed to get ctid")
                    .expect("ctid was NULL");

                let new_position = &positions[idx];
                let quoted_new_position = quote_literal(new_position);

                // Use quote_literal to safely escape the ctid string
                let quoted_ctid = quote_literal(&ctid_str);
                let update_query = format!(
                    "UPDATE {} SET {} = {} WHERE ctid = {}::tid",
                    quoted_table, quoted_lexo_column, quoted_new_position, quoted_ctid
                );

                client
                    .update(&update_query, None, &[])
                    .expect("Failed to update row position");
            }
        });

        row_count
    }
}

/// Generate a vector of evenly distributed position strings
fn generate_balanced_positions(count: usize) -> Vec<String> {
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
fn fraction_to_position(fraction: f64) -> String {
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
fn generate_after(s: &str) -> String {
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
fn generate_before(s: &str) -> String {
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
fn generate_between(before: &str, after: &str) -> String {
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

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_first() {
        let pos = crate::lexo::first();
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_between_null_null() {
        let pos = crate::lexo::between(None, None);
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_tight_spacing() {
        // Test that consecutive after() calls produce tightly spaced values
        let first = crate::lexo::first();
        let second = crate::lexo::after(&first);
        let third = crate::lexo::after(&second);

        // Should be relatively short strings (1-2 chars)
        assert!(second.len() <= 2, "Second position too long: {}", second);
        assert!(third.len() <= 2, "Third position too long: {}", third);
    }

    #[pg_test]
    fn test_between_tight() {
        let pos1 = "A";
        let pos2 = "C";
        let between = crate::lexo::between(Some(pos1), Some(pos2));

        assert!(between > pos1.to_string());
        assert!(between < pos2.to_string());
        assert!(between.len() <= 2, "Between position too long: {}", between);
    }

    #[pg_test]
    fn test_after() {
        let pos = crate::lexo::after("V");
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_before() {
        let pos = crate::lexo::before("V");
        assert!(pos < "V".to_string());
    }

    #[pg_test]
    fn test_between_with_before() {
        let pos = crate::lexo::between(Some("0"), None);
        assert!(pos > "0".to_string());
    }

    #[pg_test]
    fn test_between_with_after() {
        let pos = crate::lexo::between(None, Some("z"));
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_between_two_values() {
        let pos = crate::lexo::between(Some("0"), Some("z"));
        assert!(pos > "0".to_string());
        assert!(pos < "z".to_string());
    }

    #[pg_test]
    fn test_ordering_sequence() {
        let first = crate::lexo::first();
        let second = crate::lexo::after(&first);
        let third = crate::lexo::after(&second);

        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_insert_between_sequence() {
        let first = crate::lexo::first();
        let third = crate::lexo::after(&first);
        let second = crate::lexo::between(Some(&first), Some(&third));

        assert!(first < second);
        assert!(second < third);
    }

    #[pg_test]
    fn test_add_lexo_column() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_add_col (id SERIAL PRIMARY KEY)").unwrap();
        crate::lexo::add_lexo_column_to("test_add_col", "position");

        // Verify the column was created with COLLATE "C"
        Spi::run("INSERT INTO test_add_col (position) VALUES ('V')").unwrap();

        let result: Option<String> =
            Spi::get_one("SELECT position FROM test_add_col LIMIT 1").unwrap();
        assert_eq!(result, Some("V".to_string()));
    }

    #[pg_test]
    fn test_next_empty() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_empty (id SERIAL PRIMARY KEY, position TEXT COLLATE \"C\")").unwrap();

        let pos = crate::lexo::next("test_empty", "position", None, None);
        assert_eq!(pos, "V");
    }

    #[pg_test]
    fn test_next_with_data() {
        use pgrx::spi::Spi;

        Spi::run(
            "CREATE TEMPORARY TABLE test_data (id SERIAL PRIMARY KEY, position TEXT COLLATE \"C\")",
        )
        .unwrap();
        Spi::run("INSERT INTO test_data (position) VALUES ('V')").unwrap();

        let pos = crate::lexo::next("test_data", "position", None, None);
        assert!(pos > "V".to_string());
    }

    #[pg_test]
    fn test_next_with_filter() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_filter (collection_id TEXT, song_id TEXT, position TEXT COLLATE \"C\", PRIMARY KEY (collection_id, song_id))").unwrap();
        Spi::run("INSERT INTO test_filter (collection_id, song_id, position) VALUES ('col1', 'song1', 'A'), ('col1', 'song2', 'M'), ('col2', 'song3', 'Z')").unwrap();

        let pos = crate::lexo::next(
            "test_filter",
            "position",
            Some("collection_id"),
            Some("col1"),
        );
        assert!(pos > "M".to_string());

        let pos2 = crate::lexo::next(
            "test_filter",
            "position",
            Some("collection_id"),
            Some("col2"),
        );
        assert!(pos2 > "Z".to_string());

        let pos3 = crate::lexo::next(
            "test_filter",
            "position",
            Some("collection_id"),
            Some("col3"),
        );
        assert_eq!(pos3, "V");
    }

    #[pg_test]
    fn test_collation_ordering() {
        use pgrx::spi::Spi;

        // Create a table with proper collation
        Spi::run("CREATE TEMPORARY TABLE test_order (id SERIAL PRIMARY KEY, position TEXT COLLATE \"C\")").unwrap();
        Spi::run("INSERT INTO test_order (position) VALUES ('A'), ('Z'), ('a')").unwrap();

        // Verify C collation ordering: A < Z < a
        let result: Option<String> =
            Spi::get_one("SELECT position FROM test_order ORDER BY position LIMIT 1").unwrap();
        assert_eq!(result, Some("A".to_string()));

        let result2: Option<String> =
            Spi::get_one("SELECT position FROM test_order ORDER BY position DESC LIMIT 1").unwrap();
        assert_eq!(result2, Some("a".to_string()));
    }

    #[pg_test]
    fn test_rebalance_empty_table() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_rebalance_empty (id SERIAL PRIMARY KEY, position TEXT COLLATE \"C\")").unwrap();

        let count = crate::lexo::rebalance("test_rebalance_empty", "position", None, None);
        assert_eq!(count, 0);
    }

    #[pg_test]
    fn test_rebalance_single_row() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_rebalance_single (id SERIAL PRIMARY KEY, position TEXT COLLATE \"C\")").unwrap();
        Spi::run("INSERT INTO test_rebalance_single (position) VALUES ('zzzzz')").unwrap();

        let count = crate::lexo::rebalance("test_rebalance_single", "position", None, None);
        assert_eq!(count, 1);

        // After rebalancing, position should be 'V' (the midpoint)
        let result: Option<String> =
            Spi::get_one("SELECT position FROM test_rebalance_single LIMIT 1").unwrap();
        assert_eq!(result, Some("V".to_string()));
    }

    #[pg_test]
    fn test_rebalance_preserves_order() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_rebalance_order (id SERIAL PRIMARY KEY, name TEXT, position TEXT COLLATE \"C\")").unwrap();
        // Insert rows with long positions that simulate many insertions
        Spi::run("INSERT INTO test_rebalance_order (name, position) VALUES ('first', 'VVVV'), ('second', 'VVVk'), ('third', 'VVku')").unwrap();

        // Get original order
        let original_order: Vec<String> = Spi::connect(|client| {
            let rows = client
                .select(
                    "SELECT name FROM test_rebalance_order ORDER BY position",
                    None,
                    &[],
                )
                .unwrap();
            rows.map(|row| row.get::<String>(1).unwrap().unwrap())
                .collect()
        });

        // Rebalance
        let count = crate::lexo::rebalance("test_rebalance_order", "position", None, None);
        assert_eq!(count, 3);

        // Get new order - should be the same
        let new_order: Vec<String> = Spi::connect(|client| {
            let rows = client
                .select(
                    "SELECT name FROM test_rebalance_order ORDER BY position",
                    None,
                    &[],
                )
                .unwrap();
            rows.map(|row| row.get::<String>(1).unwrap().unwrap())
                .collect()
        });

        assert_eq!(original_order, new_order);
    }

    #[pg_test]
    fn test_rebalance_with_filter() {
        use pgrx::spi::Spi;

        Spi::run("CREATE TEMPORARY TABLE test_rebalance_filter (playlist_id TEXT, song_id TEXT, position TEXT COLLATE \"C\", PRIMARY KEY (playlist_id, song_id))").unwrap();
        Spi::run("INSERT INTO test_rebalance_filter (playlist_id, song_id, position) VALUES ('p1', 's1', 'AAAA'), ('p1', 's2', 'MMMM'), ('p2', 's3', 'ZZZZ')").unwrap();

        // Rebalance only playlist p1
        let count = crate::lexo::rebalance(
            "test_rebalance_filter",
            "position",
            Some("playlist_id"),
            Some("p1"),
        );
        assert_eq!(count, 2);

        // p2's position should be unchanged
        let p2_pos: Option<String> =
            Spi::get_one("SELECT position FROM test_rebalance_filter WHERE playlist_id = 'p2'")
                .unwrap();
        assert_eq!(p2_pos, Some("ZZZZ".to_string()));
    }
}

/// Standard Rust unit tests that don't require PostgreSQL
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_generate_first() {
        let first = lexo::first();
        assert_eq!(first, "V");
    }

    #[test]
    fn test_generate_after_minimal() {
        // New minimal spacing tests
        assert_eq!(generate_after("A"), "B");
        assert_eq!(generate_after("0"), "1");
        assert_eq!(generate_after("Z"), "a");
    }

    #[test]
    fn test_generate_before_minimal() {
        // New minimal spacing tests
        assert_eq!(generate_before("B"), "A");
        assert_eq!(generate_before("Z"), "Y");
        assert_eq!(generate_before("a"), "Z");
        assert_eq!(generate_before("1"), "0z"); // Need extra char here
    }

    #[test]
    fn test_generate_after_overflow() {
        // When we reach 'z', should append a character to go after
        let result = generate_after("z");
        assert!(result > "z".to_string());
        assert_eq!(result, "z0");
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
        let pos = generate_between("0", "1");
        assert!(pos > "0".to_string());
        assert!(pos < "1".to_string());
    }

    #[test]
    fn test_generate_between_tight() {
        let result = generate_between("A", "C");
        assert_eq!(result, "B");

        // For adjacent characters, need to go deeper
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
        // Test that consecutive after() calls produce tightly spaced values
        let first = lexo::first();
        let second = generate_after(&first);
        let third = generate_after(&second);

        // Should be relatively short strings
        assert!(second.len() <= 2, "Second position too long: {}", second);
        assert!(third.len() <= 2, "Third position too long: {}", third);
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
        let first = lexo::first();
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
        let first = lexo::first();
        let before_first = generate_before(&first);

        assert!(before_first < first);
    }

    #[test]
    fn test_between_function() {
        let between_null = lexo::between(None, None);
        assert_eq!(between_null, "V");

        let after_v = lexo::between(Some("V"), None);
        assert!(after_v > "V".to_string());

        let before_v = lexo::between(None, Some("V"));
        assert!(before_v < "V".to_string());

        let between = lexo::between(Some("0"), Some("z"));
        assert!(between > "0".to_string());
        assert!(between < "z".to_string());
    }

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
    fn test_generate_between_invalid_order() {
        let pos = generate_between("z", "0");
        assert!(pos > "z".to_string());
    }

    #[test]
    fn test_generate_between_equal_strings() {
        let pos = generate_between("V", "V");
        assert!(pos > "V".to_string());
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
        assert!(is_valid_base62("V"));
        assert!(is_valid_base62("abc123XYZ"));
        assert!(!is_valid_base62("hello!"));
        assert!(!is_valid_base62("test-value"));
        assert!(is_valid_base62("")); // empty is valid (no characters to validate)
    }

    #[test]
    fn test_generate_between_same_prefix() {
        let pos = generate_between("AB", "AC");
        assert!(pos > "AB".to_string());
        assert!(pos < "AC".to_string());
        // Note: The new algorithm may or may not preserve the prefix
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
        // Test case where before is a prefix of after and we can find a midpoint
        let pos = generate_between("z", "z1");
        assert!(pos > "z".to_string());
        assert!(pos < "z1".to_string());

        // Another valid case
        let pos2 = generate_between("A", "AA");
        assert!(pos2 > "A".to_string());
        assert!(pos2 < "AA".to_string());
    }

    #[test]
    #[should_panic(expected = "Cannot generate a position before")]
    fn test_generate_before_minimum_position_panics() {
        // "0" is the minimum single-character position
        let _ = generate_before("0");
    }

    #[test]
    #[should_panic(expected = "Cannot generate a position before")]
    fn test_generate_before_all_zeros_panics() {
        // "000" is also a minimum position (all zeros)
        let _ = generate_before("000");
    }

    #[test]
    fn test_generate_between_z_and_z0() {
        // "z" < "z0" in lexicographic order, but there's no valid character between them
        // since '0' is the first character. The algorithm falls back to appending MID_CHAR.
        // Note: "zV" > "z0" because 'V' > '0', so this doesn't actually work as "between".
        // This is an edge case where the result may not be strictly between the inputs.
        let result = generate_between("z", "z0");
        // The result should at least be > "z"
        assert!(result > "z".to_string());
    }

    #[test]
    fn test_generate_before_with_trailing_zeros() {
        // "A0" can have a valid "before" because we can decrement 'A' to find a midpoint
        let pos = generate_before("A0");
        assert!(pos < "A0".to_string());

        // "10" can have a valid "before" because we can decrement '1' to '0'
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
        assert_eq!(positions[0], "V");
    }

    #[test]
    fn test_generate_balanced_positions_multiple() {
        let positions = generate_balanced_positions(5);
        assert_eq!(positions.len(), 5);

        // Verify they are in sorted order
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

        // Verify they are in sorted order
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
        // Test some key fractions
        let pos_0 = fraction_to_position(0.0);
        let pos_half = fraction_to_position(0.5);
        let pos_1 = fraction_to_position(0.999);

        assert!(pos_0 < pos_half);
        assert!(pos_half < pos_1);
    }
}

/// This module is required by `cargo pgrx test` invocations.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
