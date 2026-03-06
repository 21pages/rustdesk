use hbb_common::{
    config::{Config, LocalConfig},
    log,
    sodiumoxide::{
        base64::{decode as b64decode, encode as b64encode, Variant},
        crypto::sign,
    },
};
use serde_derive::{Deserialize, Serialize};
use std::sync::Mutex;

// Controller: persistent Ed25519 keypair cache (sk, pk)
static USER_KEY_PAIR: Mutex<Option<(Vec<u8>, Vec<u8>)>> = Mutex::new(None);

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

pub fn get_user_key_pair() -> (Vec<u8>, Vec<u8>) {
    let mut lock = USER_KEY_PAIR.lock().unwrap();
    if let Some((sk, pk)) = lock.as_ref() {
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
                *lock = Some((sk, pk));
                return lock.as_ref().unwrap().clone();
            }
        }
        log::warn!("Invalid user key pair in local config, regenerate");
    }

    let (pk, sk) = sign::gen_keypair();
    let pk = pk.as_ref().to_vec();
    let sk = sk.as_ref().to_vec();
    let pk_b64 = b64encode(&pk, Variant::Original);
    let sk_b64 = b64encode(sk.as_slice(), Variant::Original);
    if let Ok(user_key_pair) = serde_json::to_string(&(sk_b64, pk_b64.clone())) {
        LocalConfig::set_option(USER_KEY_PAIR_KEY.to_owned(), user_key_pair);
    } else {
        log::error!("Failed to serialize user key pair");
    }
    *lock = Some((sk, pk));
    lock.as_ref().unwrap().clone()
}

pub fn sign_easy_access_challenge(challenge: &[u8], user_sk: &[u8]) -> Option<Vec<u8>> {
    let sk = sign::SecretKey::from_slice(user_sk)?;
    Some(sign::sign_detached(challenge, &sk).as_ref().to_vec())
}
