use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, OsRng},
};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::api::error::ApiError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy)]
pub enum HashDomain {
    Session,
    Csrf,
    OAuthState,
}

impl HashDomain {
    fn prefix(self) -> &'static str {
        match self {
            Self::Session => "session:",
            Self::Csrf => "csrf:",
            Self::OAuthState => "oauth_state:",
        }
    }
}

pub fn random_token(bytes: usize) -> String {
    let mut token = vec![0_u8; bytes];
    OsRng.fill_bytes(&mut token);
    general_purpose::URL_SAFE_NO_PAD.encode(token)
}

pub fn token_hash(secret: &[u8], domain: HashDomain, token: &str) -> String {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(secret).expect("HMAC accepts keys of any length");
    mac.update(domain.prefix().as_bytes());
    mac.update(token.as_bytes());
    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

pub fn constant_time_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

#[allow(deprecated)]
pub fn encrypt_token(key: &[u8; 32], plaintext: &str) -> Result<String, ApiError> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce = [0_u8; 24];
    OsRng.fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|_| ApiError::internal())?;

    Ok(format!(
        "v1:{}:{}",
        general_purpose::URL_SAFE_NO_PAD.encode(nonce),
        general_purpose::URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

#[allow(dead_code)]
#[allow(deprecated)]
pub fn decrypt_token(key: &[u8; 32], envelope: &str) -> Result<String, ApiError> {
    let mut parts = envelope.split(':');
    let version = parts.next();
    let nonce = parts.next();
    let ciphertext = parts.next();

    if version != Some("v1") || parts.next().is_some() {
        return Err(ApiError::internal());
    }

    let nonce = general_purpose::URL_SAFE_NO_PAD
        .decode(nonce.unwrap_or_default())
        .map_err(|_| ApiError::internal())?;
    let ciphertext = general_purpose::URL_SAFE_NO_PAD
        .decode(ciphertext.unwrap_or_default())
        .map_err(|_| ApiError::internal())?;

    let cipher = XChaCha20Poly1305::new(key.into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| ApiError::internal())?;

    String::from_utf8(plaintext).map_err(|_| ApiError::internal())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_hashes_use_domain_separation() {
        let secret = b"01234567890123456789012345678901";
        let token = "same-token";

        assert_ne!(
            token_hash(secret, HashDomain::Session, token),
            token_hash(secret, HashDomain::Csrf, token)
        );
    }

    #[test]
    fn encryption_envelope_does_not_contain_plaintext() {
        let key = [7_u8; 32];
        let encrypted = encrypt_token(&key, "secret-token").expect("encrypts");

        assert!(encrypted.starts_with("v1:"));
        assert!(!encrypted.contains("secret-token"));
        assert_eq!(
            decrypt_token(&key, &encrypted).expect("decrypts"),
            "secret-token"
        );
    }
}
