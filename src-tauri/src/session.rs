use crate::auth::{self, Config, Identity, Tokens};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use zeroize::{Zeroize, Zeroizing};

#[derive(Serialize, Deserialize)]
struct SavedSession {
    refresh_token: String,
    account_hash: String,
    project: String,
    #[serde(default)]
    identity: Option<Identity>,
}
impl Drop for SavedSession {
    fn drop(&mut self) {
        self.refresh_token.zeroize();
    }
}

pub fn exists(root: &Path) -> bool {
    root.join("session.bin").is_file()
}
fn account_hash(uid: &str) -> String {
    format!("{:x}", Sha256::digest(uid.as_bytes()))
}

// DPAPI CurrentUser: credentials stay bound to this Windows user's profile.
#[cfg(windows)]
fn protect(bytes: &mut [u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::{
        core::w,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::Cryptography::{
                CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
            },
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes
            .len()
            .try_into()
            .map_err(|_| "SESSION_STORAGE_FAILED")?,
        pbData: bytes.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let result = if encrypt {
            CryptProtectData(
                &input,
                w!("OpenRideSync Viewer"),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_| {
            if encrypt {
                "SESSION_STORAGE_FAILED"
            } else {
                "SESSION_UNREADABLE"
            }
        })?;
        let buffer = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = buffer.to_vec();
        buffer.zeroize();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(result)
    }
}
#[cfg(not(windows))]
fn protect(_: &mut [u8], _: bool) -> Result<Vec<u8>, String> {
    Err("SESSION_PLATFORM_UNSUPPORTED".into())
}

pub fn save(
    root: &Path,
    config: &Config,
    identity: &Identity,
    refresh_token: String,
) -> Result<(), String> {
    let session = SavedSession {
        refresh_token,
        account_hash: account_hash(&identity.uid),
        identity: Some(identity.clone()),
        project: config.firebase["projectId"]
            .as_str()
            .ok_or("SERVICE_CONFIG_MISSING")?
            .into(),
    };
    let mut bytes =
        Zeroizing::new(serde_json::to_vec(&session).map_err(|_| "SESSION_STORAGE_FAILED")?);
    let encrypted = protect(&mut bytes, true)?;
    std::fs::create_dir_all(root).map_err(|_| "SESSION_STORAGE_FAILED")?;
    let pending = root.join("session.pending");
    std::fs::write(&pending, encrypted).map_err(|_| "SESSION_STORAGE_FAILED")?;
    std::fs::rename(pending, root.join("session.bin")).map_err(|_| "SESSION_STORAGE_FAILED")?;
    Ok(())
}

fn refresh_error(status: reqwest::StatusCode, body: &serde_json::Value) -> &'static str {
    // A connection/server failure does not invalidate the saved login.
    let code = body["error"]["message"].as_str().unwrap_or("");
    if status == reqwest::StatusCode::BAD_REQUEST
        && matches!(
            code,
            "TOKEN_EXPIRED"
                | "INVALID_REFRESH_TOKEN"
                | "USER_DISABLED"
                | "USER_NOT_FOUND"
                | "INVALID_GRANT"
                | "MISSING_REFRESH_TOKEN"
        )
    {
        "SESSION_EXPIRED"
    } else {
        "SESSION_OFFLINE"
    }
}

pub fn identity(root: &Path) -> Result<Option<Identity>, String> {
    if !exists(root) {
        return Ok(None);
    }
    let saved = load(root)?;
    Ok(saved.identity.clone())
}
fn load(root: &Path) -> Result<SavedSession, String> {
    let mut encrypted =
        std::fs::read(root.join("session.bin")).map_err(|_| "SESSION_UNREADABLE")?;
    let bytes = Zeroizing::new(protect(&mut encrypted, false)?);
    serde_json::from_slice(&bytes).map_err(|_| "SESSION_UNREADABLE".into())
}
pub fn forget(root: &Path) -> Result<(), String> {
    for name in ["session.bin", "session.pending"] {
        match std::fs::remove_file(root.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("SESSION_STORAGE_FAILED".into()),
        }
    }
    Ok(())
}
pub async fn refresh(root: &Path, config: &Config) -> Result<(Tokens, Identity), String> {
    let saved = load(root)?;
    if config.firebase["projectId"] != saved.project {
        return Err("SESSION_EXPIRED".into());
    }
    let key = config.firebase["apiKey"]
        .as_str()
        .ok_or("SERVICE_CONFIG_MISSING")?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "SESSION_OFFLINE")?;
    let response = client
        .post("https://securetoken.googleapis.com/v1/token")
        .query(&[("key", key)])
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", saved.refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|_| "SESSION_OFFLINE")?;
    let status = response.status();
    let body: serde_json::Value = response.json().await.map_err(|_| "SESSION_OFFLINE")?;
    if !status.is_success() {
        return Err(refresh_error(status, &body).into());
    }
    let token = body["id_token"]
        .as_str()
        .ok_or("SESSION_OFFLINE")?
        .to_string();
    let uid = auth::verify(&token, config).await?;
    if account_hash(&uid.uid) != saved.account_hash {
        return Err("SESSION_EXPIRED".into());
    }
    let refresh_token = body["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("SESSION_OFFLINE")?
        .to_string();
    save(root, config, &uid, refresh_token.clone())?;
    Ok((
        Tokens {
            token,
            refresh_token,
        },
        uid,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn persisted_login_can_be_replaced_and_reopened() {
        let root =
            std::env::temp_dir().join(format!("roadsync-session-test-{}", uuid::Uuid::new_v4()));
        let config = Config {
            firebase: serde_json::json!({"projectId":"synthetic"}),
            honda_api_key: String::new(),
        };
        save(
            &root,
            &config,
            &Identity {
                uid: "synthetic-user".into(),
                email: Some("rider@example.test".into()),
                name: Some("Test Rider".into()),
            },
            "first-synthetic-token".into(),
        )
        .unwrap();
        save(
            &root,
            &config,
            &Identity {
                uid: "synthetic-user".into(),
                email: Some("rider@example.test".into()),
                name: Some("Test Rider".into()),
            },
            "rotated-synthetic-token".into(),
        )
        .unwrap();
        let mut encrypted = std::fs::read(root.join("session.bin")).unwrap();
        assert!(!encrypted.windows(9).any(|w| w == b"synthetic"));
        let bytes = protect(&mut encrypted, false).unwrap();
        let saved: SavedSession = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(saved.refresh_token, "rotated-synthetic-token");
        assert_eq!(saved.account_hash, account_hash("synthetic-user"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn dpapi_roundtrip_and_tamper_rejection() {
        let original = b"synthetic-refresh-token";
        let mut encrypted = protect(&mut original.to_vec(), true).unwrap();
        assert!(!encrypted.windows(original.len()).any(|w| w == original));
        assert_eq!(protect(&mut encrypted, false).unwrap(), original);
        let index = encrypted.len() / 2;
        encrypted[index] ^= 0xff;
        assert!(protect(&mut encrypted, false).is_err());
    }
    #[test]
    fn offline_does_not_mean_expired() {
        use reqwest::StatusCode;
        use serde_json::json;
        assert_eq!(
            refresh_error(
                StatusCode::BAD_REQUEST,
                &json!({"error":{"message":"TOKEN_EXPIRED"}})
            ),
            "SESSION_EXPIRED"
        );
        assert_eq!(
            refresh_error(StatusCode::SERVICE_UNAVAILABLE, &json!({})),
            "SESSION_OFFLINE"
        );
        assert_eq!(
            refresh_error(StatusCode::TOO_MANY_REQUESTS, &json!({})),
            "SESSION_OFFLINE"
        );
    }
}
