use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as Base64};
use hmac::{Hmac, Mac};
use serde::{Serialize, de::DeserializeOwned};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("invalid payload")]
    InvalidPayload,
    #[error("invalid hmac key")]
    HmacKeyError,
    #[error("invalid token format")]
    InvalidFormat,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("wrong token type")]
    WrongType,
    #[error("token expired")]
    Expired,
}

pub trait Token: Serialize + DeserializeOwned {
    const TYPE: &'static str;

    fn exp(&self) -> i64;

    fn typ(&self) -> &str {
        Self::TYPE
    }

    fn validate(&self, now_ts: i64) -> Result<(), TokenError> {
        if self.typ() != Self::TYPE {
            return Err(TokenError::WrongType);
        }
        if self.exp() < now_ts {
            return Err(TokenError::Expired);
        }
        Ok(())
    }
}

pub struct TokenSigner {
    key: Vec<u8>,
}

impl TokenSigner {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    pub fn sign_token<T: Token>(&self, token: &T) -> Result<String, TokenError> {
        let payload = serde_json::to_vec(&token).map_err(|_| TokenError::InvalidPayload)?;
        let payload_b64 = Base64.encode(payload);

        let sig = self.hmac(payload_b64.as_bytes())?;
        let sig_b64 = Base64.encode(sig);

        Ok(format!("{payload_b64}.{sig_b64}"))
    }

    pub fn verify_token<T: Token>(&self, token: &str, now_s: i64) -> Result<T, TokenError> {
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(TokenError::InvalidFormat)?;

        let token_sig = Base64
            .decode(sig_b64.as_bytes())
            .map_err(|_| TokenError::InvalidSignature)?;

        self.verify_hmac(payload_b64.as_bytes(), &token_sig)?;

        let payload = Base64
            .decode(payload_b64.as_bytes())
            .map_err(|_| TokenError::InvalidPayload)?;

        let token: T = serde_json::from_slice(&payload).map_err(|_| TokenError::InvalidPayload)?;

        token.validate(now_s)?;

        Ok(token)
    }

    fn hmac(&self, data: &[u8]) -> Result<[u8; 32], TokenError> {
        let mut mac =
            HmacSha256::new_from_slice(&self.key).map_err(|_| TokenError::HmacKeyError)?;
        mac.update(data);
        let bytes = mac.finalize().into_bytes();
        Ok(bytes.into())
    }

    fn verify_hmac(&self, data: &[u8], expected_sig: &[u8]) -> Result<(), TokenError> {
        let mut mac =
            HmacSha256::new_from_slice(&self.key).map_err(|_| TokenError::HmacKeyError)?;
        mac.update(data);
        mac.verify_slice(expected_sig)
            .map_err(|_| TokenError::InvalidSignature)
    }
}
