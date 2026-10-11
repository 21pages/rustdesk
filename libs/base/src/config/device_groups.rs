use hbb_common::{
    anyhow::anyhow,
    compress::decompress,
    config::{Config, APP_NAME},
    password_security::symmetric_crypt,
    ResultType,
};
use serde_json::Value;
use std::{fs, io::ErrorKind};

pub fn load_cache() -> ResultType<Option<String>> {
    let path = Config::path(format!("{}_group", APP_NAME.read().unwrap().as_str()));
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    decode_cache(&data)
}

fn decode_cache(data: &[u8]) -> ResultType<Option<String>> {
    let data = symmetric_crypt(data, false)
        .map_err(|_| anyhow!("Failed to decrypt device group cache"))?;
    let data = decompress(&data);
    let value: Value = serde_json::from_slice(&data)?;
    let has_memberships = value
        .get("peers")
        .and_then(Value::as_array)
        .map_or(false, |peers| {
            peers.iter().any(|peer| {
                peer.get("device_group_names")
                    .and_then(Value::as_array)
                    .map_or(false, |groups| !groups.is_empty())
            })
        });
    if has_memberships {
        Ok(Some(String::from_utf8(data)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::compress::compress;
    use serde_json::json;

    fn encode(value: &Value) -> Vec<u8> {
        symmetric_crypt(&compress(&serde_json::to_vec(value).unwrap()), true).unwrap()
    }

    #[test]
    fn retains_memberships_through_encrypted_cache() {
        let value = json!({
            "access_token": "test-token",
            "peers": [{"id": "123456789", "device_group_name": "Primary", "device_group_names": ["Primary", "Support"]}],
            "device_groups": [{"name": "Support"}]
        });
        let loaded = decode_cache(&encode(&value)).unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&loaded).unwrap(), value);
    }

    #[test]
    fn legacy_cache_uses_the_existing_loader() {
        for peer in [
            json!({"id": "123456789", "device_group_name": "Primary"}),
            json!({"id": "123456789", "device_group_names": []}),
        ] {
            assert!(decode_cache(&encode(&json!({"peers": [peer]})))
                .unwrap()
                .is_none());
        }
    }
}
