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

pub enum CredentialsError {
    UsernameInvalidChars,
    UsernameTooShort,
    UsernameTooLong,
    PasswordInvalidChars,
    PasswordTooShort,
    PasswordTooLong,
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
    type Error = CredentialsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let len = value.chars().count();

        if len < Self::MIN_USERNAME_LENGTH {
            return Err(CredentialsError::UsernameTooShort);
        }

        if len > Self::MAX_USERNAME_LENGTH {
            return Err(CredentialsError::UsernameTooLong);
        }

        let valid = value.chars().all(|c| c.is_ascii_alphanumeric());

        if !valid {
            return Err(CredentialsError::UsernameInvalidChars);
        }

        Ok(UserName(value))
    }
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
    type Error = CredentialsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let len = value.chars().count();

        if len < Self::MIN_PASSWORD_LENGTH {
            return Err(CredentialsError::PasswordTooShort);
        }
        if len > Self::MAX_PASSWORD_LENGTH {
            return Err(CredentialsError::PasswordTooLong);
        }

        if value.chars().any(|c| c.is_control()) {
            return Err(CredentialsError::PasswordInvalidChars);
        }

        Ok(UserPassword(value))
    }
}
