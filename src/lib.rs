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
pub mod operations;
mod schema;

/// The `lexo` schema contains all functions and types for lexicographic ordering.
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
#[pg_schema]
pub mod lexo {
    use pgrx::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::cmp::Ordering;
    use std::fmt;
    use std::hash::{Hash, Hasher};
    use std::str::FromStr;

    use crate::operations::{MID_CHAR, is_valid_base62};

    /// A lexicographic rank type for ordering items in PostgreSQL.
    ///
    /// `LexoRank` wraps a Base62-encoded string that maintains lexicographic ordering.
    /// This type can be used as a column type in PostgreSQL tables to efficiently
    /// manage the order of rows without needing to update other rows when inserting.
    ///
    /// # Example
    /// ```sql
    /// CREATE TABLE items (
    ///     id SERIAL PRIMARY KEY,
    ///     position lexo.lexorank NOT NULL
    /// );
    ///
    /// -- Insert items using the lexo functions
    /// INSERT INTO items (position) VALUES (lexo.first());
    /// ```
    // Note: PostgresOrd and PostgresHash are intentionally NOT derived here because
    // pgrx does not properly schema-qualify operator class definitions for types
    // defined within a #[pg_schema] module (see https://github.com/pgcentralfoundation/pgrx/issues/2134).
    // Users should use COLLATE "C" when ordering by lexo.lexorank columns.
    #[derive(Debug, Clone, Serialize, Deserialize, PostgresType, PostgresEq)]
    #[inoutfuncs]
    pub struct LexoRank {
        value: String,
    }

    impl LexoRank {
        /// Creates a new LexoRank from a string value.
        ///
        /// # Arguments
        /// * `value` - A Base62-encoded string
        ///
        /// # Panics
        /// Panics if the value contains invalid Base62 characters.
        pub fn new(value: String) -> Self {
            if !value.is_empty() && !is_valid_base62(&value) {
                pgrx::error!(
                    "Invalid LexoRank value '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                    value
                );
            }
            Self { value }
        }

        /// Creates a new LexoRank from a string reference.
        ///
        /// # Arguments
        /// * `value` - A Base62-encoded string reference
        ///
        /// # Panics
        /// Panics if the value contains invalid Base62 characters.
        pub fn from_str_ref(value: &str) -> Self {
            Self::new(value.to_string())
        }

        /// Returns the first/initial LexoRank value.
        pub fn first() -> Self {
            Self {
                value: MID_CHAR.to_string(),
            }
        }

        /// Returns the inner string value.
        pub fn as_str(&self) -> &str {
            &self.value
        }

        /// Consumes self and returns the inner string value.
        pub fn into_inner(self) -> String {
            self.value
        }

        /// Returns true if the LexoRank is empty.
        pub fn is_empty(&self) -> bool {
            self.value.is_empty()
        }
    }

    impl Default for LexoRank {
        fn default() -> Self {
            Self::first()
        }
    }

    impl fmt::Display for LexoRank {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.value)
        }
    }

    impl FromStr for LexoRank {
        type Err = &'static str;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if !s.is_empty() && !is_valid_base62(s) {
                return Err(
                    "Invalid LexoRank: must contain only Base62 characters (0-9, A-Z, a-z)",
                );
            }
            Ok(Self {
                value: s.to_string(),
            })
        }
    }

    impl PartialEq for LexoRank {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    impl Eq for LexoRank {}

    impl PartialOrd for LexoRank {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for LexoRank {
        fn cmp(&self, other: &Self) -> Ordering {
            self.value.cmp(&other.value)
        }
    }

    impl Hash for LexoRank {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.value.hash(state);
        }
    }

    impl InOutFuncs for LexoRank {
        fn input(input: &core::ffi::CStr) -> Self
        where
            Self: Sized,
        {
            let s = input.to_str().expect("Invalid UTF-8 in LexoRank input");
            Self::from_str(s).expect("Invalid LexoRank value")
        }

        fn output(&self, buffer: &mut pgrx::StringInfo) {
            buffer.push_str(&self.value);
        }
    }

    impl From<String> for LexoRank {
        fn from(value: String) -> Self {
            Self::new(value)
        }
    }

    impl From<&str> for LexoRank {
        fn from(value: &str) -> Self {
            Self::from_str_ref(value)
        }
    }

    impl From<LexoRank> for String {
        fn from(rank: LexoRank) -> Self {
            rank.into_inner()
        }
    }

    impl AsRef<str> for LexoRank {
        fn as_ref(&self) -> &str {
            &self.value
        }
    }

    // Re-export all schema functions
    pub use crate::schema::*;
}

// Re-export the LexoRank type for public use from the lexo schema
pub use lexo::LexoRank;

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
