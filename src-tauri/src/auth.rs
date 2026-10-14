use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{get, post},
    Json, Router,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub firebase: Value,
    pub honda_api_key: String,
}
pub fn parse_config(text: &str) -> Result<Config, String> {
    if text.len() > 16_384 {
        return Err("SERVICE_CONFIG_INVALID".into());
    }
    let value: Config = serde_json::from_str(text).map_err(|_| "SERVICE_CONFIG_INVALID")?;
    for key in ["projectId", "apiKey", "authDomain"] {
        if !value.firebase[key].as_str().is_some_and(|s| {
            !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
        }) {
            return Err("SERVICE_CONFIG_INVALID".into());
        }
    }
    if value.honda_api_key.trim().is_empty()
        || value.honda_api_key.len() > 4096
        || !value.honda_api_key.is_ascii()
        || value.honda_api_key.chars().any(char::is_control)
    {
        return Err("SERVICE_CONFIG_INVALID".into());
    }
    Ok(value)
}
pub fn config(root: &Path) -> Result<Config, String> {
    if root.join("service-config.json").exists() {
        return local_config(root);
    }
    parse_config(include_str!(concat!(
        env!("OUT_DIR"),
        "/service-config.json"
    )))
    .map_err(|_| "SERVICE_CONFIG_MISSING".into())
}
fn local_config(root: &Path) -> Result<Config, String> {
    let path = root.join("service-config.json");
    if std::fs::metadata(&path)
        .map_err(|_| "SERVICE_CONFIG_MISSING")?
        .len()
        > 16_384
    {
        return Err("SERVICE_CONFIG_INVALID".into());
    }
    parse_config(&std::fs::read_to_string(path).map_err(|_| "SERVICE_CONFIG_MISSING")?)
}
pub fn import_config(root: &Path, text: &str) -> Result<(), String> {
    let value = parse_config(text)?;
    // Changing projects while signed in could mix accounts and cached journeys.
    if crate::session::exists(root) {
        return Err("SERVICE_CONFIG_SIGN_OUT_FIRST".into());
    }
    std::fs::create_dir_all(root).map_err(|_| "SERVICE_CONFIG_SAVE_FAILED")?;
    let pending = root.join("service-config.pending");
    std::fs::write(
        &pending,
        serde_json::to_vec_pretty(&value).map_err(|_| "SERVICE_CONFIG_INVALID")?,
    )
    .map_err(|_| "SERVICE_CONFIG_SAVE_FAILED")?;
    std::fs::rename(pending, root.join("service-config.json"))
        .map_err(|_| "SERVICE_CONFIG_SAVE_FAILED".into())
}
pub struct Tokens {
    pub token: String,
    pub refresh_token: String,
}
impl Drop for Tokens {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.token.zeroize();
        self.refresh_token.zeroize();
    }
}
#[derive(Clone)]
struct Callback {
    nonce: String,
    origin: String,
    sender: Arc<Mutex<Option<oneshot::Sender<Tokens>>>>,
}
async fn receive(
    State(state): State<Callback>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if headers.get("origin").and_then(|h| h.to_str().ok()) != Some(state.origin.as_str())
        || headers.get("host").and_then(|h| h.to_str().ok())
            != Some(state.origin.trim_start_matches("http://"))
        || headers.get("x-viewer-nonce").and_then(|h| h.to_str().ok()) != Some(state.nonce.as_str())
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let token = body["token"]
        .as_str()
        .filter(|s| s.len() <= 8192 && s.split('.').count() == 3)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let refresh_token = body["refreshToken"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 8192)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let sender = state
        .sender
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .take()
        .ok_or(StatusCode::CONFLICT)?;
    sender
        .send(Tokens {
            token: token.to_string(),
            refresh_token: refresh_token.to_string(),
        })
        .map_err(|_| StatusCode::GONE)?;
    Ok(Json(json!({"accepted":true})))
}
pub async fn login(config: &Config, notify: impl Fn(String)) -> Result<Tokens, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "LOGIN_PORT_UNAVAILABLE")?;
    let origin = format!(
        "http://localhost:{}",
        listener
            .local_addr()
            .map_err(|_| "LOGIN_PORT_UNAVAILABLE")?
            .port()
    );
    let nonce = uuid::Uuid::new_v4().to_string();
    let path = format!("/login/{nonce}");
    let public = json!({"firebase":config.firebase,"nonce":nonce});
    let html = include_str!("../login.html")
        .replace("__CONFIG__", &public.to_string().replace('<', "\\u003c"));
    let (tx, rx) = oneshot::channel();
    let state = Callback {
        nonce,
        origin: origin.clone(),
        sender: Arc::new(Mutex::new(Some(tx))),
    };
    let app = Router::new()
        .route(&path, get(move || async move { Html(html) }))
        .route("/token", post(receive))
        .layer(DefaultBodyLimit::max(24000))
        .with_state(state);
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    let url = format!("{origin}{path}");
    notify(url.clone());
    let opened = webbrowser::open(&url).is_ok();
    let result = if opened {
        tokio::time::timeout(std::time::Duration::from_secs(600), rx)
            .await
            .map_err(|_| "LOGIN_TIMEOUT".to_string())
            .and_then(|r| r.map_err(|_| "LOGIN_CANCELLED".into()))
    } else {
        Err("BROWSER_OPEN_FAILED".into())
    };
    let _ = stop.send(());
    let _ = server.await;
    result
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub uid: String,
    pub email: Option<String>,
    pub name: Option<String>,
}
pub async fn verify(token: &str, config: &Config) -> Result<Identity, String> {
    let header = decode_header(token).map_err(|_| "AUTH_INVALID")?;
    if header.alg != Algorithm::RS256 {
        return Err("AUTH_ALGORITHM_INVALID".into());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "AUTH_CLIENT_FAILED")?;
    let jwks: jsonwebtoken::jwk::JwkSet=client.get("https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com")
        .send().await.map_err(|_|"AUTH_KEYS_UNAVAILABLE")?.error_for_status().map_err(|_|"AUTH_KEYS_UNAVAILABLE")?
        .json().await.map_err(|_|"AUTH_KEYS_INVALID")?;
    let key = jwks
        .find(header.kid.as_deref().ok_or("AUTH_KEY_MISSING")?)
        .ok_or("AUTH_KEY_UNKNOWN")?;
    let project = config.firebase["projectId"]
        .as_str()
        .ok_or("SERVICE_CONFIG_MISSING")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[project]);
    validation.set_issuer(&[format!("https://securetoken.google.com/{project}")]);
    validation.leeway = 0;
    validation.set_required_spec_claims(&["exp", "iat", "iss", "aud", "sub", "auth_time"]);
    let claims = decode::<Value>(
        token,
        &DecodingKey::from_jwk(key).map_err(|_| "AUTH_KEY_INVALID")?,
        &validation,
    )
    .map_err(|_| "AUTH_INVALID")?
    .claims;
    let now = chrono::Utc::now().timestamp();
    let uid = claims["sub"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .ok_or("AUTH_SUBJECT_INVALID")?;
    for field in ["iat", "auth_time"] {
        if claims[field]
            .as_i64()
            .filter(|t| *t > 0 && *t <= now + 30)
            .is_none()
        {
            return Err("AUTH_TIME_INVALID".into());
        }
    }
    if claims["firebase"]["sign_in_provider"] != "google.com" {
        return Err("GOOGLE_PROVIDER_REQUIRED".into());
    }
    Ok(Identity {
        uid: uid.into(),
        email: claims["email"].as_str().map(str::to_owned),
        name: claims["name"].as_str().map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_service_configuration_is_validated_and_never_overwrites_a_session() {
        let root =
            std::env::temp_dir().join(format!("openridesync-config-test-{}", uuid::Uuid::new_v4()));
        let original = json!({"firebase":{"projectId":"synthetic","apiKey":"synthetic-api-key","authDomain":"synthetic.firebaseapp.com"},"hondaApiKey":"synthetic-client-key"});
        assert!(local_config(&root).is_err());
        for invalid in [
            json!(null),
            json!({}),
            json!({"firebase":{},"hondaApiKey":""}),
        ] {
            assert!(import_config(&root, &invalid.to_string()).is_err());
        }
        assert!(parse_config(&" ".repeat(16385)).is_err());
        import_config(&root, &original.to_string()).unwrap();
        assert_eq!(config(&root).unwrap().firebase["projectId"], "synthetic");
        let saved = std::fs::read(root.join("service-config.json")).unwrap();
        let mut invalid = original.clone();
        invalid["hondaApiKey"] = json!("invalid\nheader");
        assert!(import_config(&root, &invalid.to_string()).is_err());
        invalid = original.clone();
        invalid["refreshToken"] = json!("must-not-be-imported");
        assert!(import_config(&root, &invalid.to_string()).is_err());
        std::fs::write(root.join("session.bin"), b"synthetic").unwrap();
        assert_eq!(
            import_config(&root, &original.to_string()).unwrap_err(),
            "SERVICE_CONFIG_SIGN_OUT_FIRST"
        );
        assert_eq!(
            std::fs::read(root.join("service-config.json")).unwrap(),
            saved
        );
        crate::account::apply(&root, "all").unwrap();
        assert!(local_config(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn callback_requires_origin_nonce_and_single_use() {
        let (tx, rx) = oneshot::channel();
        let state = Callback {
            nonce: "test-nonce".into(),
            origin: "http://localhost:1234".into(),
            sender: Arc::new(Mutex::new(Some(tx))),
        };
        let mut headers = HeaderMap::new();
        headers.insert("host", "localhost:1234".parse().unwrap());
        headers.insert("origin", "https://example.com".parse().unwrap());
        headers.insert("x-viewer-nonce", "test-nonce".parse().unwrap());
        let body = json!({"token":"test.test.test","refreshToken":"synthetic-refresh"});
        assert_eq!(
            receive(State(state.clone()), headers.clone(), Json(body.clone()))
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
        headers.insert("origin", "http://localhost:1234".parse().unwrap());
        headers.insert("x-viewer-nonce", "wrong".parse().unwrap());
        assert_eq!(
            receive(State(state.clone()), headers.clone(), Json(body.clone()))
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
        headers.insert("x-viewer-nonce", "test-nonce".parse().unwrap());
        assert!(
            receive(State(state.clone()), headers.clone(), Json(body.clone()))
                .await
                .is_ok()
        );
        assert_eq!(rx.await.unwrap().token.as_str(), "test.test.test");
        assert_eq!(
            receive(State(state), headers, Json(body))
                .await
                .unwrap_err(),
            StatusCode::CONFLICT
        );
    }
}
