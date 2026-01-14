use thiserror::Error;

const MIN_ALIAS_LENGTH: usize = 6;
const MAX_ALIAS_LENGTH: usize = 20;

#[derive(Debug)]
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
    pub fn parse(input: &str) -> Result<Self, AliasParseError> {
        if input.len() < MIN_ALIAS_LENGTH {
            return Err(AliasParseError::TooShort);
        }
        if input.len() > MAX_ALIAS_LENGTH {
            return Err(AliasParseError::TooLong);
        }
        if input.contains(|c: char| !c.is_alphanumeric()) {
            return Err(AliasParseError::InvalidCharacters);
        }
        Ok(Self(input.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allowed_aliases() {
        let aliases = ["abcdef", "abcde1234567890", "abcde12345678901234"];
        for alias in aliases {
            let result = Alias::parse(alias);
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
            "abcde",
            "abcde12345678901234567890",
            "abcde1234567890!@#$%",
            "ab-cde",
            "ab_cde",
            "ab.cde",
            "ab&cde",
            "ab cde",
            "ab/cde",
        ];
        for alias in aliases {
            let result = Alias::parse(alias);
            assert!(
                result.is_err(),
                "{} should not be allowed, instead: {:?}",
                alias,
                result
            );
        }
    }
}
