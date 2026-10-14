use crate::{data, session};
use serde_json::{json, Value};
use std::path::Path;

pub fn info(root: &Path) -> Result<Value, String> {
    let snapshot = data::load(root)?;
    let identity = session::identity(root);
    let cache = snapshot.as_ref();
    Ok(
        json!({"identity":identity.as_ref().ok().and_then(|i|i.as_ref()),"identityUnavailable":identity.is_err(),"hasSavedSession":session::exists(root),
        "images":cache.and_then(|s|s["images"].as_object()).map_or(0,|i|i.len()),
        "days":cache.and_then(|s|s["summary"]["days"].as_u64()).unwrap_or(0),
        "trips":cache.and_then(|s|s["summary"]["trips"].as_u64()).unwrap_or(0)}),
    )
}
fn remove(root: &Path, name: &str) -> Result<(), String> {
    match std::fs::remove_file(root.join(name)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("CACHE_CLEAR_FAILED".into()),
    }
}
fn snapshots(root: &Path) -> Result<Vec<String>, String> {
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut names = vec![];
    for entry in std::fs::read_dir(root).map_err(|_| "CACHE_UNREADABLE")? {
        let entry = entry.map_err(|_| "CACHE_UNREADABLE")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name
            .strip_prefix("snapshot-")
            .and_then(|s| s.strip_suffix(".json"))
            .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok())
        {
            names.push(name);
        }
    }
    Ok(names)
}
pub fn apply(root: &Path, action: &str) -> Result<(), String> {
    if !matches!(action, "images" | "history" | "logout" | "all") {
        return Err("ACCOUNT_ACTION_INVALID".into());
    }
    if action == "logout" {
        return session::forget(root);
    }
    let previous = snapshots(root)?;
    if action == "all" {
        session::forget(root)?;
        for name in [
            "latest.json",
            "latest.pending",
            "last-operation.json",
            "service-config.json",
            "service-config.pending",
        ] {
            remove(root, name)?;
        }
    } else if let Some(mut snapshot) = data::load(root)? {
        if action == "images" {
            snapshot["images"] = json!({});
            snapshot["cacheState"]["imagesCleared"] = json!(true);
        } else {
            snapshot["days"] = json!([]);
            snapshot["summary"] = json!({"days":0,"trips":0,"points":0});
            snapshot["cacheState"]["historyCleared"] = json!(true);
        }
        data::save(root, &snapshot)?;
    }
    // Also remove superseded snapshots, so cleared information is not retained in an older copy.
    // Paths are fixed app-owned filenames; no recursive deletion or caller-provided paths.
    for name in previous {
        remove(root, &name)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selective_clear_logout_and_reset_preserve_unrelated_files() {
        let root = std::env::temp_dir().join(format!(
            "openridesync-account-test-{}",
            uuid::Uuid::new_v4()
        ));
        let original = json!({"version":1,"models":[{}],"images":{"test":"image-data"},"days":[{"date":"synthetic"}],"summary":{"days":1,"trips":2,"points":3}});
        data::save(&root, &original).unwrap();
        data::save(&root, &original).unwrap();
        std::fs::write(root.join("session.bin"), b"synthetic").unwrap();
        std::fs::write(root.join("unrelated-measurement.bin"), b"retain").unwrap();
        apply(&root, "images").unwrap();
        let cached = data::load(&root).unwrap().unwrap();
        assert_eq!(cached["images"], json!({}));
        assert_eq!(cached["summary"]["trips"], 2);
        assert_eq!(snapshots(&root).unwrap().len(), 1);
        assert!(session::exists(&root));
        apply(&root, "history").unwrap();
        let cached = data::load(&root).unwrap().unwrap();
        assert_eq!(cached["days"], json!([]));
        assert_eq!(cached["models"], json!([{}]));
        apply(&root, "logout").unwrap();
        assert!(!session::exists(&root));
        assert!(data::load(&root).unwrap().is_some());
        assert_eq!(
            apply(&root, "../private").unwrap_err(),
            "ACCOUNT_ACTION_INVALID"
        );
        apply(&root, "all").unwrap();
        assert!(data::load(&root).unwrap().is_none());
        assert!(snapshots(&root).unwrap().is_empty());
        assert_eq!(
            std::fs::read(root.join("unrelated-measurement.bin")).unwrap(),
            b"retain"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
