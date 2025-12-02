//! The `lexo` schema module containing all public PostgreSQL functions.
//!
//! This module provides functions for lexicographic ordering of items in PostgreSQL tables.
//! Use the `LexoRank` type with `COLLATE "C"` for proper ordering.

use pgrx::prelude::*;
use pgrx::spi::{Spi, quote_identifier, quote_literal};

use crate::lexorank::LexoRank;
use crate::operations::{
    MID_CHAR, generate_after, generate_balanced_positions, generate_before,
    generate_between as gen_between, is_valid_base62,
};

/// Returns the first position for a new ordered list.
///
/// # Returns
/// The initial LexoRank position ('H')
///
/// # Example
/// ```sql
/// SELECT lexo.first();  -- Returns 'H'
/// INSERT INTO items (position) VALUES (lexo.first());
/// ```
#[pg_extern(schema = "lexo")]
pub fn first() -> LexoRank {
    LexoRank::first()
}

/// Returns a position after the given position.
///
/// # Arguments
/// * `current` - The current position (must be valid base62)
///
/// # Returns
/// A new LexoRank that comes after `current`
///
/// # Example
/// ```sql
/// SELECT lexo.after('H');  -- Returns a position after 'H'
/// SELECT lexo.after(lexo.first());
/// ```
#[pg_extern(schema = "lexo")]
pub fn after(current: LexoRank) -> LexoRank {
    let result = generate_after(current.as_str());
    LexoRank::new(result)
}

/// Returns a position before the given position.
///
/// # Arguments
/// * `current` - The current position (must be valid base62)
///
/// # Returns
/// A new LexoRank that comes before `current`
///
/// # Example
/// ```sql
/// SELECT lexo.before('H');  -- Returns a position before 'H'
/// SELECT lexo.before(lexo.first());
/// ```
#[pg_extern(schema = "lexo")]
pub fn before(current: LexoRank) -> LexoRank {
    let result = generate_before(current.as_str());
    LexoRank::new(result)
}

/// Returns a position between two existing positions.
///
/// # Arguments
/// * `before_pos` - The position before the new position (can be NULL for beginning)
/// * `after_pos` - The position after the new position (can be NULL for end)
///
/// # Returns
/// A new LexoRank that lexicographically falls between `before_pos` and `after_pos`
///
/// # Example
/// ```sql
/// SELECT lexo.between(NULL, NULL);       -- Returns 'H' (first position)
/// SELECT lexo.between('H', NULL);        -- Returns position after 'H'
/// SELECT lexo.between(NULL, 'H');        -- Returns position before 'H'
/// SELECT lexo.between('A', 'Z');         -- Returns midpoint between 'A' and 'Z'
/// ```
#[pg_extern(schema = "lexo")]
pub fn between(before_pos: Option<LexoRank>, after_pos: Option<LexoRank>) -> LexoRank {
    let before_str = before_pos.as_ref().map(|r| r.as_str()).unwrap_or("");
    let after_str = after_pos.as_ref().map(|r| r.as_str()).unwrap_or("");

    match (before_str.is_empty(), after_str.is_empty()) {
        (true, true) => LexoRank::first(),
        (false, true) => LexoRank::new(generate_after(before_str)),
        (true, false) => LexoRank::new(generate_before(after_str)),
        (false, false) => LexoRank::new(gen_between(before_str, after_str)),
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
/// A new LexoRank after the maximum, or 'H' if table is empty
///
/// # Example
/// ```sql
/// -- Get next position for entire table
/// SELECT lexo.next('items', 'position', NULL, NULL);
///
/// -- Get next position for a specific collection
/// SELECT lexo.next('collection_songs', 'position', 'collection_id', 'abc-123');
/// ```
#[pg_extern(schema = "lexo")]
pub fn next(
    table_name: &str,
    lexo_column_name: &str,
    identifier_column_name: Option<&str>,
    identifier_value: Option<&str>,
) -> LexoRank {
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
                "SELECT MAX({} COLLATE \"C\")::text FROM {} WHERE {} = {}",
                quoted_lexo_column, quoted_table, quoted_id_column, quoted_id_value
            )
        }
        _ => {
            format!(
                "SELECT MAX({} COLLATE \"C\")::text FROM {}",
                quoted_lexo_column, quoted_table
            )
        }
    };

    let max_position: Option<String> =
        Spi::get_one(&query).expect("Failed to query table for maximum position");

    match max_position {
        Some(pos) => LexoRank::new(generate_after(&pos)),
        None => LexoRank::first(),
    }
}

/// Adds a lexo position column to an existing table.
///
/// The column will be of type `lexo.lexorank` to ensure proper
/// lexicographic ordering with the custom type.
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
/// -- ALTER TABLE items ADD COLUMN position lexo.lexorank;
/// ```
#[pg_extern(schema = "lexo")]
pub fn add_lexo_column_to(table_name: &str, column_name: &str) {
    let quoted_table = if let Some((schema, table)) = table_name.split_once('.') {
        format!("{}.{}", quote_identifier(schema), quote_identifier(table))
    } else {
        quote_identifier(table_name)
    };

    let quoted_column = quote_identifier(column_name);

    let query = format!(
        "ALTER TABLE {} ADD COLUMN {} lexo.lexorank",
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
#[pg_extern(schema = "lexo")]
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
    let positions = generate_balanced_positions(row_count as usize);

    // Build query to get all rows ordered by current position, using ctid as text
    let select_query = match (&key_column_name, &key_value) {
        (Some(key_col), Some(key_val)) => {
            let quoted_key_column = quote_identifier(key_col);
            let quoted_key_value = quote_literal(key_val);
            format!(
                "SELECT ctid::text FROM {} WHERE {} = {} ORDER BY {}::text COLLATE \"C\"",
                quoted_table, quoted_key_column, quoted_key_value, quoted_lexo_column
            )
        }
        _ => format!(
            "SELECT ctid::text FROM {} ORDER BY {}::text COLLATE \"C\"",
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
                "UPDATE {} SET {} = {}::lexo.lexorank WHERE ctid = {}::tid",
                quoted_table, quoted_lexo_column, quoted_new_position, quoted_ctid
            );

            client
                .update(&update_query, None, &[])
                .expect("Failed to update row position");
        }
    });

    row_count
}

// Legacy TEXT-based functions for backwards compatibility

/// Returns a position after the given position (TEXT version for backwards compatibility).
///
/// # Arguments
/// * `current` - The current position as TEXT (must be valid base62)
///
/// # Returns
/// A new position string that comes after `current`
///
/// # Example
/// ```sql
/// SELECT lexo.after_text('H');  -- Returns a position after 'H'
/// ```
#[pg_extern(schema = "lexo")]
pub fn after_text(current: &str) -> String {
    if !is_valid_base62(current) {
        pgrx::error!(
            "Invalid position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
            current
        );
    }
    generate_after(current)
}

/// Returns a position before the given position (TEXT version for backwards compatibility).
///
/// # Arguments
/// * `current` - The current position as TEXT (must be valid base62)
///
/// # Returns
/// A new position string that comes before `current`
///
/// # Example
/// ```sql
/// SELECT lexo.before_text('H');  -- Returns a position before 'H'
/// ```
#[pg_extern(schema = "lexo")]
pub fn before_text(current: &str) -> String {
    if !is_valid_base62(current) {
        pgrx::error!(
            "Invalid position '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
            current
        );
    }
    generate_before(current)
}

/// Returns the first position as TEXT (for backwards compatibility).
///
/// # Returns
/// The initial position ('H') as TEXT
///
/// # Example
/// ```sql
/// SELECT lexo.first_text();  -- Returns 'H'
/// ```
#[pg_extern(schema = "lexo")]
pub fn first_text() -> String {
    MID_CHAR.to_string()
}

/// Returns a position between two existing positions (TEXT version for backwards compatibility).
///
/// # Arguments
/// * `before_pos` - The position before the new position (can be NULL for beginning)
/// * `after_pos` - The position after the new position (can be NULL for end)
///
/// # Returns
/// A new position string that lexicographically falls between `before_pos` and `after_pos`
///
/// # Example
/// ```sql
/// SELECT lexo.between_text(NULL, NULL);       -- Returns 'H' (first position)
/// SELECT lexo.between_text('H', NULL);        -- Returns position after 'H'
/// SELECT lexo.between_text(NULL, 'H');        -- Returns position before 'H'
/// SELECT lexo.between_text('A', 'Z');         -- Returns midpoint between 'A' and 'Z'
/// ```
#[pg_extern(schema = "lexo")]
pub fn between_text(before_pos: Option<&str>, after_pos: Option<&str>) -> String {
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
