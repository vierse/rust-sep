mod data;
mod sign;

use std::{borrow::Cow, marker::PhantomData};

use data::*;
pub use sign::*;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::domain::LinkId;

pub type OwnerToken = SignedToken<OwnerType, { 24 * 60 * 60 }, 50>;
pub type UnlockToken = SignedToken<UnlockType, { 60 * 60 }, 10>;

pub trait TokenType {
    const TYPE: &'static str;
}
pub struct OwnerType;
impl TokenType for OwnerType {
    const TYPE: &'static str = "owner";
}

pub struct UnlockType;
impl TokenType for UnlockType {
    const TYPE: &'static str = "unlock";
}

pub trait Token: Serialize + DeserializeOwned {
    const TYPE: &'static str;

    fn exp(&self) -> i64;
    fn typ(&self) -> &str;
    fn contains(&self, id: LinkId, now_s: i64) -> bool;

    fn validate(&self, now_s: i64) -> Result<(), TokenError> {
        if self.typ() != Self::TYPE {
            return Err(TokenError::WrongType);
        }
        if self.exp() <= now_s {
            return Err(TokenError::Expired);
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct SignedToken<T: TokenType, const TTL_SECS: i64, const CAP: usize> {
    typ: Cow<'static, str>,
    data: TokenData<TTL_SECS, CAP>,
    #[serde(skip)]
    _pd: PhantomData<T>,
}

impl<T: TokenType, const TTL_SECS: i64, const CAP: usize> Default
    for SignedToken<T, TTL_SECS, CAP>
{
    fn default() -> Self {
        Self {
            typ: Cow::Borrowed(T::TYPE),
            data: TokenData::default(),
            _pd: PhantomData,
        }
    }
}

impl<T: TokenType, const TTL_SECS: i64, const CAP: usize> SignedToken<T, TTL_SECS, CAP> {
    pub fn remaining_secs(&self, id: LinkId, now_s: i64) -> Option<i64> {
        self.data
            .iter()
            .find(|e| e.id() == id)
            .and_then(|e| (e.exp() > now_s).then_some(e.exp() - now_s))
    }

    pub fn update(&mut self, id: LinkId, now_s: i64) {
        self.data.update(id, now_s);
    }

    pub fn typ(&self) -> &str {
        &self.typ
    }

    pub fn exp(&self) -> i64 {
        self.data.iter().map(|e| e.exp()).max().unwrap_or(0)
    }

    pub fn ttl() -> i64 {
        TTL_SECS
    }
}

impl<T: TokenType, const TTL_SECS: i64, const CAP: usize> Token for SignedToken<T, TTL_SECS, CAP> {
    const TYPE: &'static str = T::TYPE;

    fn typ(&self) -> &str {
        self.typ()
    }

    fn exp(&self) -> i64 {
        self.exp()
    }

    fn contains(&self, id: LinkId, now_s: i64) -> bool {
        self.data.iter().any(|e| e.id() == id && e.exp() > now_s)
    }
}
