//! LexoRank type module for PostgreSQL.
//!
//! This module defines the `LexoRank` custom PostgreSQL type that wraps
//! Base62-encoded lexicographic position strings with validation and comparison support.

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
#[derive(
    Debug, Clone, Serialize, Deserialize, PostgresType, PostgresEq, PostgresOrd, PostgresHash,
)]
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
            return Err("Invalid LexoRank: must contain only Base62 characters (0-9, A-Z, a-z)");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexorank_new() {
        let rank = LexoRank::new("H".to_string());
        assert_eq!(rank.as_str(), "H");
    }

    #[test]
    fn test_lexorank_first() {
        let rank = LexoRank::first();
        assert_eq!(rank.as_str(), "H");
    }

    #[test]
    fn test_lexorank_empty() {
        let rank = LexoRank::new(String::new());
        assert!(rank.is_empty());
    }

    #[test]
    fn test_lexorank_ordering() {
        let a = LexoRank::new("A".to_string());
        let b = LexoRank::new("B".to_string());
        let z = LexoRank::new("Z".to_string());

        assert!(a < b);
        assert!(b < z);
        assert!(a < z);
    }

    #[test]
    fn test_lexorank_equality() {
        let a1 = LexoRank::new("ABC".to_string());
        let a2 = LexoRank::new("ABC".to_string());
        let b = LexoRank::new("DEF".to_string());

        assert_eq!(a1, a2);
        assert_ne!(a1, b);
    }

    #[test]
    fn test_lexorank_from_str() {
        let rank: LexoRank = "Hello".parse().unwrap();
        assert_eq!(rank.as_str(), "Hello");
    }

    #[test]
    fn test_lexorank_from_str_invalid() {
        let result: Result<LexoRank, _> = "Hello!".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_lexorank_display() {
        let rank = LexoRank::new("TestValue".to_string());
        assert_eq!(format!("{}", rank), "TestValue");
    }

    #[test]
    fn test_lexorank_into_string() {
        let rank = LexoRank::new("Value".to_string());
        let s: String = rank.into();
        assert_eq!(s, "Value");
    }

    #[test]
    fn test_lexorank_from_string() {
        let rank: LexoRank = "Value".to_string().into();
        assert_eq!(rank.as_str(), "Value");
    }

    #[test]
    fn test_lexorank_from_str_ref() {
        let rank: LexoRank = "Value".into();
        assert_eq!(rank.as_str(), "Value");
    }
}
