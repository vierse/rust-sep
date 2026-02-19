use thiserror::Error;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Alias(String);

#[derive(Error, Debug)]
pub enum AliasParseError {
    #[error("too short")]
    TooShort,
    #[error("too long")]
    TooLong,
    #[error("contains invalid characters")]
    InvalidCharacters,
}

impl Alias {
    pub const MIN_ALIAS_LENGTH: usize = 4;
    pub const MAX_ALIAS_LENGTH: usize = 64;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Alias {
    type Error = AliasParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let len = value.chars().count();

        if len < Self::MIN_ALIAS_LENGTH {
            return Err(AliasParseError::TooShort);
        }

        if len > Self::MAX_ALIAS_LENGTH {
            return Err(AliasParseError::TooLong);
        }

        let valid = value.chars().all(|c| c.is_ascii_alphanumeric());

        if !valid {
            return Err(AliasParseError::InvalidCharacters);
        }

        Ok(Alias(value))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allowed_aliases() {
        let aliases = ["abcdef", "abcde1234567890", "abcde12345678901234"];
        for alias in aliases {
            let result: Result<Alias, _> = alias.to_string().try_into();
            assert!(
                result.is_ok(),
                "{} should be allowed, instead: {:?}",
                alias,
                result
            );
        }
    }

    #[test]
    fn disallowed_aliases() {
        let aliases = [
            "",
            "a",
            "abcde1234567890!@#$%",
            "ab-cde",
            "ab_cde",
            "ab.cde",
            "ab&cde",
            "ab cde",
            "ab/cde",
        ];
        for alias in aliases {
            let result: Result<Alias, _> = alias.to_string().try_into();
            assert!(
                result.is_err(),
                "{} should not be allowed, instead: {:?}",
                alias,
                result
            );
        }
    }
}
