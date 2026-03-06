use hbb_common::{
    config::{Config, LocalConfig},
    log,
    sodiumoxide::{
        base64::{decode as b64decode, encode as b64encode, Variant},
        crypto::sign,
    },
};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};

// Controller: persistent Ed25519 keypair cache (sk, pk)
lazy_static::lazy_static! {
    static ref USER_KEY_PAIR: Mutex<HashMap<String, (Vec<u8>, Vec<u8>)>> =
        Mutex::new(HashMap::new());
}

const CONFIG_KEY: &str = "easy-access-approved-keys";
const USER_KEY_PAIR_KEY: &str = "user-key-pair";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedKey {
    pub user_guid: String,
    pub public_key: String,
}

/// Check if a public key (base64-encoded) is in the locally approved list.
pub fn is_approved(pk_base64: &str) -> bool {
    let keys = get_approved_keys();
    keys.iter().any(|k| k.public_key == pk_base64)
}

/// Get all approved keys from local storage (reads directly from config each time).
pub fn get_approved_keys() -> Vec<ApprovedKey> {
    load_from_config()
}

/// Save approved keys to local storage.
pub fn set_approved_keys(keys: Vec<ApprovedKey>) {
    save_to_config(&keys);
}

fn load_from_config() -> Vec<ApprovedKey> {
    let json = Config::get_option(CONFIG_KEY);
    if json.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str(&json) {
        Ok(keys) => keys,
        Err(e) => {
            log::warn!("Failed to parse easy access approved keys: {}", e);
            Vec::new()
        }
    }
}

fn save_to_config(keys: &[ApprovedKey]) {
    match serde_json::to_string(keys) {
        Ok(json) => {
            Config::set_option(CONFIG_KEY.to_owned(), json);
        }
        Err(e) => {
            log::error!("Failed to serialize easy access approved keys: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Controller side: session keypair management
// ---------------------------------------------------------------------------

pub fn get_user_key_pair(username: &str) -> (Vec<u8>, Vec<u8>) {
    let mut lock = USER_KEY_PAIR.lock().unwrap();
    if let Some((sk, pk)) = lock.get(username) {
        return (sk.clone(), pk.clone());
    }

    let user_key_pair = LocalConfig::get_option(USER_KEY_PAIR_KEY);
    if let Ok((sk_b64, pk_b64)) = serde_json::from_str::<(String, String)>(&user_key_pair) {
        if let (Ok(sk), Ok(pk)) = (
            b64decode(&sk_b64, Variant::Original),
            b64decode(&pk_b64, Variant::Original),
        ) {
            if sign::PublicKey::from_slice(&pk).is_some()
                && sign::SecretKey::from_slice(&sk).is_some()
            {
                lock.insert(username.to_string(), (sk, pk));
                return lock.get(username).unwrap().clone();
            }
        }
        log::warn!("Invalid user key pair in local config, regenerate");
    }

    let (pk, sk) = sign::gen_keypair();
    let pk = pk.as_ref().to_vec();
    let sk = sk.as_ref().to_vec();
    let pk_b64 = b64encode(&pk, Variant::Original);
    let sk_b64 = b64encode(sk.as_slice(), Variant::Original);
    let mut old: HashMap<String, (String, String)> =
        serde_json::from_str(&LocalConfig::get_option(USER_KEY_PAIR_KEY)).unwrap_or_default();
    old.insert(username.to_string(), (sk_b64, pk_b64));
    LocalConfig::set_option(
        USER_KEY_PAIR_KEY.to_owned(),
        serde_json::to_string(&old).unwrap_or_default(),
    );
    lock.insert(username.to_string(), (sk, pk));
    lock.get(username).unwrap().clone()
}

pub fn sign_easy_access_challenge(challenge: &[u8], user_sk: &[u8]) -> Option<Vec<u8>> {
    let sk = sign::SecretKey::from_slice(user_sk)?;
    Some(sign::sign_detached(challenge, &sk).as_ref().to_vec())
}

#[cfg(test)]
mod tests {
    use super::sign_easy_access_challenge;
    use hbb_common::sodiumoxide::crypto::sign;

    #[test]
    fn user_sk_signs_challenge_and_user_pk_verifies_it() {
        // Controller side: generate user key pair and sign challenge with sk.
        let (user_pk, user_sk) = sign::gen_keypair();
        let challenge = b"easy-access-challenge";
        let sig_bytes = sign_easy_access_challenge(challenge, user_sk.as_ref())
            .expect("signature should be generated with a valid user sk");

        // Controlled side: verify the signature with user pk.
        let sig = sign::Signature::from_bytes(&sig_bytes).expect("signature bytes should be valid");
        assert!(sign::verify_detached(&sig, challenge, &user_pk));
    }
}
