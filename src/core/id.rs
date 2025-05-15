//! Id implementation for notelog

use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{IdError, NotelogError, Result};

/// An opaque wrapper type that represents a valid Id
///
/// An Id is a base36 string of length 16 (using characters 0-9 and a-z).
/// Uppercase characters are automatically converted to lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    /// Create a new Id from a string, validating it in the process
    pub fn new(input: &str) -> Result<Self> {
        // Trim the input and convert to lowercase
        let processed_input = input.trim().to_lowercase();

        // Check if the processed input is empty
        if processed_input.is_empty() {
            return Err(NotelogError::IdError(IdError::Empty));
        }

        // Check if the processed input has the correct length
        if processed_input.len() != 16 {
            return Err(NotelogError::IdError(IdError::InvalidLength(
                processed_input.len(),
            )));
        }

        // Check if the processed input contains only valid base36 characters (0-9, a-z)
        if !processed_input
            .chars()
            .all(|c| c.is_ascii_digit() || (c.is_ascii_lowercase() && c.is_ascii_alphabetic()))
        {
            return Err(NotelogError::IdError(IdError::InvalidCharacters(
                processed_input.to_string(),
            )));
        }

        Ok(Id(processed_input))
    }

    /// Get the Id as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Generate a random base36 Id
    fn generate_random() -> Self {
        const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        const ID_LENGTH: usize = 16;

        let mut rng = rng();
        let id: String = (0..ID_LENGTH)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        // This should never fail since we're generating a valid ID
        Id(id)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::generate_random()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Id {
    type Err = NotelogError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Id::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_new() {
        // Valid IDs
        assert_eq!(
            Id::new("0123456789abcdef").unwrap().as_str(),
            "0123456789abcdef"
        );
        assert_eq!(
            Id::new("abcdefghijklmnop").unwrap().as_str(),
            "abcdefghijklmnop"
        );
        assert_eq!(
            Id::new("a0b1c2d3e4f5g6h7").unwrap().as_str(),
            "a0b1c2d3e4f5g6h7"
        );

        // Test uppercase conversion
        assert_eq!(
            Id::new("0123456789ABCDEF").unwrap().as_str(),
            "0123456789abcdef"
        );
        assert_eq!(
            Id::new("ABCDEFGHIJKLMNOP").unwrap().as_str(),
            "abcdefghijklmnop"
        );

        // Test trimming
        assert_eq!(
            Id::new(" 0123456789abcdef ").unwrap().as_str(),
            "0123456789abcdef"
        );
        assert_eq!(
            Id::new("\t0123456789abcdef\n").unwrap().as_str(),
            "0123456789abcdef"
        );

        // Test mixed case and trimming
        assert_eq!(
            Id::new(" 0123456789ABCDEF ").unwrap().as_str(),
            "0123456789abcdef"
        );

        // Invalid IDs
        assert!(matches!(
            Id::new("").unwrap_err(),
            NotelogError::IdError(IdError::Empty)
        ));
        assert!(matches!(
            Id::new("   ").unwrap_err(),
            NotelogError::IdError(IdError::Empty)
        ));
        assert!(matches!(
            Id::new("abc").unwrap_err(),
            NotelogError::IdError(IdError::InvalidLength(3))
        ));
        assert!(matches!(
            Id::new("0123456789abcde!").unwrap_err(),
            NotelogError::IdError(IdError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn test_id_display() {
        let id = Id::new("0123456789abcdef").unwrap();
        assert_eq!(format!("{}", id), "0123456789abcdef");
    }

    #[test]
    fn test_id_default() {
        let id1 = Id::default();
        let id2 = Id::default();

        // Verify the length is correct
        assert_eq!(id1.as_str().len(), 16);
        assert_eq!(id2.as_str().len(), 16);

        // Verify they're different (this could theoretically fail, but it's extremely unlikely)
        assert_ne!(id1, id2);

        // Verify they contain only valid characters
        assert!(
            id1.as_str()
                .chars()
                .all(|c| c.is_ascii_digit() || (c.is_ascii_lowercase() && c.is_ascii_alphabetic()))
        );
        assert!(
            id2.as_str()
                .chars()
                .all(|c| c.is_ascii_digit() || (c.is_ascii_lowercase() && c.is_ascii_alphabetic()))
        );
    }

    #[test]
    fn test_id_from_str() {
        // Valid IDs
        let id1: Id = "0123456789abcdef".parse().unwrap();
        assert_eq!(id1.as_str(), "0123456789abcdef");

        // Test uppercase conversion
        let id2: Id = "0123456789ABCDEF".parse().unwrap();
        assert_eq!(id2.as_str(), "0123456789abcdef");

        // Test trimming
        let id3: Id = " 0123456789abcdef ".parse().unwrap();
        assert_eq!(id3.as_str(), "0123456789abcdef");

        // Invalid IDs
        let err1 = "".parse::<Id>();
        assert!(err1.is_err());

        let err2 = "abc".parse::<Id>();
        assert!(err2.is_err());

        let err3 = "0123456789abcde!".parse::<Id>();
        assert!(err3.is_err());
    }
}
