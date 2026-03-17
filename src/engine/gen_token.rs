use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

pub fn generate_token(user_id: &Uuid, secret: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(user_id.as_bytes());

    let result = mac.finalize().into_bytes();
    let sig = general_purpose::STANDARD.encode(result);

    format!("{}.{}", user_id, sig)
}

pub fn verify_token(token: &str, secret: &[u8]) -> Option<Uuid> {
    let parts = token.split(".").collect::<Vec<&str>>();
    if parts.len() != 2 {
        return None;
    }

    let user_id = Uuid::parse_str(parts[0]).ok()?;
    let sig = general_purpose::STANDARD.decode(parts[1]).ok()?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(user_id.as_bytes());

    if mac.verify_slice(&sig).is_ok() {
        Some(user_id)
    } else {
        None
    }
}
