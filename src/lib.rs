//! pg_lexo: PostgreSQL extension for lexicographic ordering.
//!
//! This extension provides efficient lexicographic ordering capabilities for PostgreSQL,
//! allowing you to insert items between any two positions without updating other rows.
//!
//! # Usage
//!
//! The extension provides a custom `lexo` type and functions:
//!
//! ```sql
//! -- Create a table with a lexo column
//! CREATE TABLE items (
//!     id SERIAL PRIMARY KEY,
//!     position lexo NOT NULL
//! );
//!
//! -- Insert using the lexo functions
//! INSERT INTO items (position) VALUES (lexo_first());
//! INSERT INTO items (position) VALUES (lexo_after(lexo_first()));
//!
//! -- Order by the position (no COLLATE needed!)
//! SELECT * FROM items ORDER BY position;
//! ```

use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use crate::operations::{MID_CHAR, is_valid_base62};

::pgrx::pg_module_magic!();

// Module declarations
pub mod operations;
mod schema;

// Re-export all functions from schema module
pub use crate::schema::*;

/// A lexicographic rank type for ordering items in PostgreSQL.
///
/// `Lexo` wraps a Base62-encoded string that maintains lexicographic ordering.
/// This type can be used as a column type in PostgreSQL tables to efficiently
/// manage the order of rows without needing to update other rows when inserting.
///
/// # Example
/// ```sql
/// CREATE TABLE items (
///     id SERIAL PRIMARY KEY,
///     position lexo NOT NULL
///);
///
/// -- Insert items using the lexo functions
/// INSERT INTO items (position) VALUES (lexo_first());
/// ```
#[derive(
    Debug, Clone, Serialize, Deserialize, PostgresType, PostgresEq, PostgresOrd, PostgresHash,
)]
#[inoutfuncs]
pub struct Lexo {
    value: String,
}

impl Lexo {
    /// Creates a new Lexo from a string value.
    ///
    /// # Arguments
    /// * `value` - A Base62-encoded string
    ///
    /// # Panics
    /// Panics if the value contains invalid Base62 characters.
    pub fn new(value: String) -> Self {
        if !value.is_empty() && !is_valid_base62(&value) {
            pgrx::error!(
                "Invalid Lexo value '{}': must contain only Base62 characters (0-9, A-Z, a-z)",
                value
            );
        }
        Self { value }
    }

    /// Creates a new Lexo from a string reference.
    ///
    /// # Arguments
    /// * `value` - A Base62-encoded string reference
    ///
    /// # Panics
    /// Panics if the value contains invalid Base62 characters.
    pub fn from_str_ref(value: &str) -> Self {
        Self::new(value.to_string())
    }

    /// Returns the first/initial Lexo value.
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

    /// Returns true if the Lexo is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl Default for Lexo {
    fn default() -> Self {
        Self::first()
    }
}

impl fmt::Display for Lexo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for Lexo {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_empty() && !is_valid_base62(s) {
            return Err("Invalid Lexo: must contain only Base62 characters (0-9, A-Z, a-z)");
        }
        Ok(Self {
            value: s.to_string(),
        })
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
    fn input(input: &core::ffi::CStr) -> Self
    where
        Self: Sized,
    {
        let s = input.to_str().expect("Invalid UTF-8 in Lexo input");
        Self::from_str(s).expect("Invalid Lexo value")
    }

    fn output(&self, buffer: &mut pgrx::StringInfo) {
        buffer.push_str(&self.value);
    }
}

impl From<String> for Lexo {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Lexo {
    fn from(value: &str) -> Self {
        Self::from_str_ref(value)
    }
}

impl From<Lexo> for String {
    fn from(rank: Lexo) -> Self {
        rank.into_inner()
    }
}

impl AsRef<str> for Lexo {
    fn as_ref(&self) -> &str {
        &self.value
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
