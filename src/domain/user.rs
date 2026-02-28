use thiserror::Error;

pub type UserId = i64;

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    name: UserName,
}

impl User {
    pub fn new(id: UserId, name: UserName) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Debug, Error)]
pub enum UsernameError {
    #[error("username is too short")]
    TooShort,
    #[error("username is too long")]
    TooLong,
    #[error("username contains invalid characters")]
    InvalidChars,
}

#[derive(Debug, Clone)]
pub struct UserName(String);

impl UserName {
    pub const MIN_USERNAME_LENGTH: usize = 4;
    pub const MAX_USERNAME_LENGTH: usize = 32;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UserName {
    type Error = UsernameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let len = value.len();

        if len < Self::MIN_USERNAME_LENGTH {
            return Err(UsernameError::TooShort);
        }

        if len > Self::MAX_USERNAME_LENGTH {
            return Err(UsernameError::TooLong);
        }

        let valid = value.chars().all(|c| c.is_ascii_alphanumeric());

        if !valid {
            return Err(UsernameError::InvalidChars);
        }

        Ok(UserName(value))
    }
}

#[derive(Debug, Error)]
pub enum UserPasswordError {
    #[error("user password is too short")]
    TooShort,
    #[error("user password is too long")]
    TooLong,
    #[error("user password contains invalid characters")]
    InvalidChars,
}

#[derive(Debug, Clone)]
pub struct UserPassword(String);

impl UserPassword {
    pub const MIN_PASSWORD_LENGTH: usize = 8;
    pub const MAX_PASSWORD_LENGTH: usize = 128;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UserPassword {
    type Error = UserPasswordError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let len = value.len();

        if len < Self::MIN_PASSWORD_LENGTH {
            return Err(UserPasswordError::TooShort);
        }
        if len > Self::MAX_PASSWORD_LENGTH {
            return Err(UserPasswordError::TooLong);
        }

        if value.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(UserPasswordError::InvalidChars);
        }

        Ok(UserPassword(value))
    }
}
