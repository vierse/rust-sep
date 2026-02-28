use std::ops::Deref;

use thiserror::Error;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LinkAlias(String);

#[derive(Debug, Error)]
pub enum LinkAliasError {
    #[error("link alias is too short")]
    TooShort,
    #[error("link alias is too long")]
    TooLong,
    #[error("link alias contains invalid characters")]
    InvalidChars,
}

impl LinkAlias {
    pub const MIN_ALIAS_LENGTH: usize = 4;
    pub const MIN_RANDOM_ALIAS_LENGTH: u8 = 6;
    pub const MAX_ALIAS_LENGTH: usize = 64;

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), LinkAliasError> {
        let len = s.len();

        if len < Self::MIN_ALIAS_LENGTH {
            return Err(LinkAliasError::TooShort);
        }

        if len > Self::MAX_ALIAS_LENGTH {
            return Err(LinkAliasError::TooLong);
        }

        if !s.chars().all(char::is_alphanumeric) {
            return Err(LinkAliasError::InvalidChars);
        }

        Ok(())
    }
}

impl Deref for LinkAlias {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl TryFrom<String> for LinkAlias {
    type Error = LinkAliasError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for LinkAlias {
    type Error = LinkAliasError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPassword(String);

#[derive(Debug, Error)]
pub enum LinkPasswordError {
    #[error("link password is too short")]
    TooShort,
    #[error("link password is too long")]
    TooLong,
    #[error("link password contains invalid characters")]
    InvalidChars,
}

impl LinkPassword {
    pub const MIN_LINK_PASSWORD_LENGTH: usize = 2;
    pub const MAX_LINK_PASSWORD_LENGTH: usize = 32;

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), LinkPasswordError> {
        let len = s.len();

        if len < Self::MIN_LINK_PASSWORD_LENGTH {
            return Err(LinkPasswordError::TooShort);
        }

        if len > Self::MAX_LINK_PASSWORD_LENGTH {
            return Err(LinkPasswordError::TooLong);
        }

        if s.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(LinkPasswordError::InvalidChars);
        }

        Ok(())
    }
}

impl Deref for LinkPassword {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl TryFrom<String> for LinkPassword {
    type Error = LinkPasswordError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for LinkPassword {
    type Error = LinkPasswordError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::validate(value)?;
        Ok(Self(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_aliases() {
        let aliases = ["abcd", "abcdef", "abcde1234567890", "abcde12345678901234"];
        for alias in aliases {
            let result = LinkAlias::try_from(alias);
            assert!(result.is_ok(), "{alias} should be allowed, got: {result:?}");
        }
    }

    #[test]
    fn disallowed_aliases() {
        let aliases = [
            "",
            "a",
            "abc",
            "abcde1234567890!@#$%",
            "ab-cde",
            "ab_cde",
            "ab.cde",
            "ab&cde",
            "ab cde",
            "ab/cde",
        ];
        for alias in aliases {
            let result = LinkAlias::try_from(alias);
            assert!(
                result.is_err(),
                "{alias} should not be allowed, got: {result:?}"
            );
        }
    }

    #[test]
    fn allowed_passwords() {
        let passwords = ["ab", "hello", "p@ssw0rd!", "1234567890"];
        for pw in passwords {
            let result = LinkPassword::try_from(pw);
            assert!(result.is_ok(), "{pw:?} should be allowed, got: {result:?}");
        }
    }

    #[test]
    fn disallowed_passwords() {
        let passwords = ["", "a", "pass word", "hi\nthere", "ok\tno"];
        for pw in passwords {
            let result = LinkPassword::try_from(pw);
            assert!(
                result.is_err(),
                "{pw:?} should not be allowed, got: {result:?}"
            );
        }

        let too_long = "a".repeat(LinkPassword::MAX_LINK_PASSWORD_LENGTH + 1);
        let result = LinkPassword::try_from(too_long);
        assert!(result.is_err());
    }
}
