//! pg_lexo: PostgreSQL extension for lexicographic ordering.
//!
//! This extension provides efficient lexicographic ordering capabilities for PostgreSQL,
//! allowing you to insert items between any two positions without updating other rows.
//!
//! # Usage
//!
//! The extension provides a custom `LexoRank` type and functions in the `lexo` schema:
//!
//! ```sql
//! -- Create a table with a lexorank column
//! CREATE TABLE items (
//!     id SERIAL PRIMARY KEY,
//!     position lexo.lexorank NOT NULL
//! );
//!
//! -- Insert using the lexo functions
//! INSERT INTO items (position) VALUES (lexo.first());
//! INSERT INTO items (position) VALUES (lexo.after(lexo.first()));
//!
//! -- Order by the position
//! SELECT * FROM items ORDER BY position;
//! ```

use pgrx::prelude::*;

::pgrx::pg_module_magic!();

// Module declarations
pub mod lexorank;
pub mod operations;
mod schema;

// Re-export the LexoRank type for public use
pub use lexorank::LexoRank;

/// The `lexo` schema contains all functions for lexicographic ordering.
///
/// Use the `lexo.lexorank` type for position columns.
///
/// # Example
/// ```sql
/// -- Create a table with a lexorank column
/// SELECT lexo.add_lexo_column_to('my_table', 'position');
///
/// -- Or manually:
/// CREATE TABLE items (
///     id SERIAL PRIMARY KEY,
///     position lexo.lexorank NOT NULL
/// );
///
/// -- Use the functions
/// INSERT INTO items (position) VALUES (lexo.first());
/// SELECT * FROM items ORDER BY position;
/// ```
/// The `lexo` schema module containing public PostgreSQL functions.
#[pg_schema]
pub mod lexo {
    // Re-export the LexoRank type in the lexo schema
    pub use crate::lexorank::LexoRank;

    // Re-export all schema functions
    pub use crate::schema::*;
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
