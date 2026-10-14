#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod account;
mod auth;
mod data;
mod session;
mod wire;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::Manager;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    phase: String,
    message: String,
    busy: bool,
    error: Option<String>,
    login_url: Option<String>,
    session: String,
    catalog: data::DownloadProgress,
    history: data::DownloadProgress,
}
struct Viewer {
    root: PathBuf,
    progress: Arc<Mutex<Progress>>,
}
fn update(state: &Arc<Mutex<Progress>>, phase: &str, message: String) {
    if let Ok(mut p) = state.lock() {
        p.phase = phase.into();
        p.message = message;
    }
}
fn failure(p: &mut Progress, code: String) {
    p.phase = "error".into();
    if matches!(
        code.as_str(),
        "SESSION_EXPIRED" | "SESSION_UNREADABLE" | "AUTH_INVALID" | "CLOUD_RPC_Unauthenticated"
    ) {
        p.session = "expired".into();
        p.message = "登入已失效，請重新登入；既有資料仍可查看".into();
    } else if matches!(code.as_str(), "SESSION_OFFLINE" | "AUTH_KEYS_UNAVAILABLE") {
        p.session = "offline".into();
        p.message = "暫時無法連線，已保留登入與本機資料".into();
    } else {
        p.message = "本次讀取未完成，既有快照仍保留".into();
    }
    p.error = Some(code);
    p.busy = false;
    p.login_url = None;
}
fn report(root: &Path, p: &Progress) {
    // No account ID, token, callback URL, or route data in the diagnostic report.
    let value = json!({"at":chrono::Utc::now().to_rfc3339(),"phase":p.phase,"session":p.session,"error":p.error});
    let _ = std::fs::write(root.join("last-operation.json"), value.to_string());
}
#[tauri::command]
fn viewer_status(state: tauri::State<Viewer>) -> Result<Progress, String> {
    state
        .progress
        .lock()
        .map(|p| p.clone())
        .map_err(|_| "STATE_UNAVAILABLE".into())
}
#[tauri::command]
fn load_snapshot(state: tauri::State<Viewer>) -> Result<Option<Value>, String> {
    data::load(&state.root)
}
#[tauri::command]
fn account_info(state: tauri::State<Viewer>) -> Result<Value, String> {
    account::info(&state.root)
}
#[tauri::command]
fn service_configured(state: tauri::State<Viewer>) -> bool {
    auth::config(&state.root).is_ok()
}
#[tauri::command]
fn import_service_config(state: tauri::State<Viewer>, text: String) -> Result<(), String> {
    let progress = state.progress.lock().map_err(|_| "STATE_UNAVAILABLE")?;
    if progress.busy {
        return Err("SYNC_ALREADY_RUNNING".into());
    }
    auth::import_config(&state.root, &text)
}
#[tauri::command]
fn account_action(state: tauri::State<Viewer>, action: String) -> Result<(), String> {
    let mut progress = state.progress.lock().map_err(|_| "STATE_UNAVAILABLE")?;
    if progress.busy {
        return Err("SYNC_ALREADY_RUNNING".into());
    }
    account::apply(&state.root, &action)?;
    if action == "logout" || action == "all" {
        progress.session = "none".into();
    }
    progress.phase = "idle".into();
    progress.error = None;
    progress.message = String::new();
    Ok(())
}
#[tauri::command]
async fn start_sync(state: tauri::State<'_, Viewer>, force_login: bool) -> Result<(), String> {
    let config = auth::config(&state.root)?;
    let progress = state.progress.clone();
    let root = state.root.clone();
    let use_saved = session::exists(&root) && !force_login;
    {
        let mut p = progress.lock().map_err(|_| "STATE_UNAVAILABLE")?;
        if p.busy {
            return Err("SYNC_ALREADY_RUNNING".into());
        }
        p.phase = if use_saved { "checking" } else { "login" }.into();
        p.message = if use_saved {
            "正在恢復登入狀態"
        } else {
            "請在瀏覽器完成 Google 登入"
        }
        .into();
        p.busy = true;
        p.error = None;
        p.login_url = None;
        p.catalog = data::DownloadProgress::default();
        p.history = data::DownloadProgress::default();
    }
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            let (tokens, uid) = if use_saved {
                session::refresh(&root, &config).await?
            } else {
                let tokens = auth::login(&config, |url| {
                    if let Ok(mut p) = progress.lock() {
                        p.login_url = Some(url);
                    }
                })
                .await?;
                update(&progress, "verifying", "正在驗證登入身分".into());
                let uid = auth::verify(&tokens.token, &config).await?;
                session::save(&root, &config, &uid, tokens.refresh_token.clone())?;
                (tokens, uid)
            };
            if let Ok(mut p) = progress.lock() {
                p.session = "active".into();
                p.login_url = None;
            }
            update(&progress, "download", "車型與旅程同時下載中".into());
            let snapshot = data::pull(&config, &tokens.token, &uid.uid, |stream, download| {
                if let Ok(mut p) = progress.lock() {
                    if stream == "catalog" {
                        p.catalog = download;
                    } else {
                        p.history = download;
                    }
                }
            })
            .await?;
            update(&progress, "saving", "保存本機快照".into());
            data::save(&root, &snapshot)?;
            Ok(())
        }
        .await;
        if let Ok(mut p) = progress.lock() {
            p.busy = false;
            p.login_url = None;
            match result {
                Ok(()) => {
                    p.phase = "done".into();
                    p.message = "車型與旅程已準備完成".into();
                    p.error = None;
                }
                Err(code) => failure(&mut p, code),
            }
            report(&root, &p);
        }
    });
    Ok(())
}
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let saved = session::exists(&root);
            let progress = Arc::new(Mutex::new(Progress {
                phase: if saved { "checking" } else { "idle" }.into(),
                message: if saved {
                    "正在恢復登入狀態"
                } else {
                    ""
                }
                .into(),
                busy: saved,
                error: None,
                login_url: None,
                session: if saved { "checking" } else { "none" }.into(),
                catalog: data::DownloadProgress::default(),
                history: data::DownloadProgress::default(),
            }));
            app.manage(Viewer {
                root: root.clone(),
                progress: progress.clone(),
            });
            if saved {
                tauri::async_runtime::spawn(async move {
                    let result = async {
                        let config = auth::config(&root)?;
                        session::refresh(&root, &config).await
                    }
                    .await;
                    if let Ok(mut p) = progress.lock() {
                        match result {
                            Ok(_) => {
                                p.phase = "idle".into();
                                p.session = "active".into();
                                p.busy = false;
                                p.message = "登入已恢復".into();
                            }
                            Err(code) => failure(&mut p, code),
                        }
                        report(&root, &p);
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            viewer_status,
            load_snapshot,
            start_sync,
            account_info,
            account_action,
            service_configured,
            import_service_config
        ])
        .run(tauri::generate_context!())
        .expect("Unable to start OpenRideSync Viewer");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Explicit live read-only check using an existing encrypted local session"]
    async fn live_parallel_read_and_account_identity() {
        let root = PathBuf::from(
            std::env::var("ROADSYNC_VIEWER_STATE").expect("Set ROADSYNC_VIEWER_STATE explicitly"),
        );
        let config = auth::config(&root).unwrap();
        let (tokens, identity) = session::refresh(&root, &config)
            .await
            .expect("Saved login refresh failed");
        let info = account::info(&root).unwrap();
        assert!(info["identity"]["uid"].as_str() == Some(identity.uid.as_str()));
        assert!(info["identity"]["email"]
            .as_str()
            .is_some_and(|s| !s.is_empty()));
        let events = Mutex::new(Vec::new());
        let snapshot = data::pull(&config, &tokens.token, &identity.uid, |stream, p| {
            events
                .lock()
                .unwrap()
                .push((stream.to_string(), p.completed, p.total));
        })
        .await
        .expect("Read-only download failed");
        let events = events.into_inner().unwrap();
        let complete = |name: &str| {
            events
                .iter()
                .position(|(s, c, t)| s == name && t.is_some_and(|total| total == *c))
                .unwrap()
        };
        let began = |name: &str| events.iter().position(|(s, _, _)| s == name).unwrap();
        assert!(began("catalog") < complete("history") && began("history") < complete("catalog"));
        for stream in ["catalog", "history"] {
            let mut previous = 0;
            for (_, completed, total) in events.iter().filter(|(s, _, _)| s == stream) {
                if let Some(total) = total {
                    assert!(*completed >= previous && completed <= total);
                    previous = *completed;
                }
            }
        }
        println!("Live read-only parallel download: {} models, {} images, {} days, {} trips, {} points; interleaved progress and encrypted account identity verified.",snapshot["models"].as_array().unwrap().len(),snapshot["images"].as_object().unwrap().len(),snapshot["summary"]["days"],snapshot["summary"]["trips"],snapshot["summary"]["points"]);
    }
    #[test]
    fn expired_and_offline_are_nonblocking_statuses() {
        let mut p = Progress {
            phase: "history".into(),
            message: String::new(),
            busy: true,
            error: None,
            login_url: None,
            session: "active".into(),
            catalog: data::DownloadProgress::default(),
            history: data::DownloadProgress::default(),
        };
        failure(&mut p, "SESSION_EXPIRED".into());
        assert_eq!(p.session, "expired");
        assert!(!p.busy);
        failure(&mut p, "SESSION_OFFLINE".into());
        assert_eq!(p.session, "offline");
        assert!(!p.busy);
    }
}
