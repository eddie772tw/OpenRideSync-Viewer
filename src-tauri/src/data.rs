use crate::wire::Client;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

fn rows<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    v[key].as_array().ok_or(format!("MISSING_{key}"))
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key].as_str().ok_or(format!("MISSING_{key}"))
}
fn integer(v: &Value) -> Result<i64, String> {
    v.as_i64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or("NUMBER_INVALID".into())
}
fn number(v: &Value) -> Result<f64, String> {
    v.as_f64()
        .filter(|n| n.is_finite() && *n >= 0.)
        .ok_or("NUMBER_INVALID".into())
}
fn labels(v: &Value, key: &str) -> Result<Vec<String>, String> {
    rows(v, key)?
        .iter()
        .map(|r| r.as_str().map(str::to_string).ok_or("LABEL_INVALID".into()))
        .collect()
}
fn model_names(model: &Value) -> Vec<String> {
    model["modelName"]
        .as_object()
        .map(|m| {
            m.values()
                .filter_map(|n| n["fullName"].as_str().map(str::to_lowercase))
                .collect()
        })
        .unwrap_or_default()
}
fn normalize(model: &Value) -> Value {
    let mut m = model.clone();
    for key in ["vehicleCodes", "images", "modelYears"] {
        if let Some(values) = m[key].as_array_mut() {
            values.sort_by_key(Value::to_string);
        }
    }
    for key in [
        "fullSupportedLanguageCodes",
        "partiallySupportedLanguageCodes",
        "notSupportedLanguageCodes",
    ] {
        if let Some(values) = m["meter"][key].as_array_mut() {
            values.sort_by_key(Value::to_string);
        }
    }
    m
}
pub fn vehicle_match(day: &Value, models: &[Value]) -> Value {
    let codes = labels(day, "vehicleSeriesCodes").unwrap_or_default();
    let names = labels(day, "vehicleDisplayName").unwrap_or_default();
    let mut candidates = Vec::new();
    let mut matched = HashSet::new();
    for model in models {
        let mut reasons = Vec::new();
        for pair in model["vehicleCodes"].as_array().into_iter().flatten() {
            for field in ["modelCode", "seriesCode"] {
                if let Some(code) = pair[field]
                    .as_str()
                    .filter(|c| codes.iter().any(|s| s == c))
                {
                    reasons.push(format!("{code} → {field}"));
                    matched.insert(code.to_string());
                }
            }
        }
        if !reasons.is_empty() {
            candidates.push(json!({"label":model["modelLabel"],"reasons":reasons}));
        }
    }
    let named: HashSet<&str> = models
        .iter()
        .filter(|m| {
            model_names(m)
                .iter()
                .any(|n| names.iter().any(|x| x.to_lowercase() == *n))
        })
        .filter_map(|m| m["modelLabel"].as_str())
        .collect();
    let mut basis = if candidates.is_empty() {
        "unmatched"
    } else {
        "code"
    };
    let mut conflict = false;
    if !candidates.is_empty() && !names.is_empty() {
        let intersection: Vec<_> = candidates
            .iter()
            .filter(|m| named.contains(m["label"].as_str().unwrap_or("")))
            .cloned()
            .collect();
        if !intersection.is_empty() {
            candidates = intersection;
            basis = "code+name";
        } else {
            conflict = true;
        }
    } else if codes.is_empty() && !named.is_empty() {
        candidates = named
            .iter()
            .map(|label| json!({"label":label,"reasons":["僅車名相同"]}))
            .collect();
        basis = "name";
    }
    candidates.sort_by_key(|m| m["label"].as_str().unwrap_or("").to_string());
    let unmatched: Vec<_> = codes.iter().filter(|c| !matched.contains(*c)).collect();
    json!({"codes":codes,"names":names,"basis":basis,"nameConflict":conflict,"unmatchedCodes":unmatched,"models":candidates})
}
pub fn project(
    days: &[Value],
    details: &HashMap<String, Value>,
    models: &[Value],
) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    let mut used = HashSet::new();
    for day in days {
        let mut trips = Vec::new();
        for uid in labels(day, "tripUuids")? {
            if !used.insert(uid.clone()) {
                return Err("REPEATED_TRIP_ID".into());
            }
            let detail = details.get(&uid).ok_or("DETAIL_MISSING")?;
            let mut points = Vec::new();
            for location in rows(detail, "tripLocations")? {
                let lat = location["coordinates"]["lat"]
                    .as_f64()
                    .ok_or("COORDINATE_INVALID")?;
                let lon = location["coordinates"]["lon"]
                    .as_f64()
                    .ok_or("COORDINATE_INVALID")?;
                let t = integer(&location["timestamp"])?;
                if !lat.is_finite()
                    || !lon.is_finite()
                    || !(-90.0..=90.).contains(&lat)
                    || !(-180.0..=180.).contains(&lon)
                    || t <= 0
                {
                    return Err("COORDINATE_INVALID".into());
                }
                points.push(json!([
                    lat,
                    lon,
                    t,
                    location["isCoarse"].as_bool().unwrap_or(false)
                ]));
            }
            points.sort_by_key(|p| p[2].as_i64().unwrap_or(0));
            trips.push(json!({"distance":number(&detail["totalDistance"])?,"avgSpeed":number(&detail["avgSpeed"])?,
                "start":points.first().map(|p|p[2].clone()),"end":points.last().map(|p|p[2].clone()),
                "coarsePoints":points.iter().filter(|p|p[3]==true).count(),"points":points}));
        }
        trips.sort_by_key(|t| t["start"].as_i64().unwrap_or(0));
        let start = integer(&day["startTime"])?;
        let end = integer(&day["endTime"])?;
        if start > end {
            return Err("TRIP_TIME_INVALID".into());
        }
        let date = chrono::DateTime::from_timestamp(start, 0)
            .ok_or("TRIP_TIME_INVALID")?
            .with_timezone(&chrono::FixedOffset::east_opt(28800).unwrap())
            .format("%Y-%m-%d")
            .to_string();
        result.push(json!({"date":date,"start":start,"end":end,"distance":number(&day["totalDistance"])?,
            "duration":integer(&day["totalTime"])?,"avgSpeed":number(&day["avgSpeed"])?,"vehicle":vehicle_match(day,models),"trips":trips}));
    }
    if used.len() != details.len() {
        return Err("DETAIL_COVERAGE_INVALID".into());
    }
    result.sort_by_key(|d| std::cmp::Reverse(d["start"].as_i64().unwrap_or(0)));
    Ok(result)
}
#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub completed: usize,
    pub total: Option<usize>,
    pub message: String,
}
impl DownloadProgress {
    fn new(completed: usize, total: Option<usize>, message: String) -> Self {
        Self {
            completed,
            total,
            message,
        }
    }
}
pub async fn pull(
    config: &crate::auth::Config,
    token: &str,
    uid: &str,
    notify: impl Fn(&str, DownloadProgress),
) -> Result<Value, String> {
    let client = Client::new(config.honda_api_key.clone()).await?;
    // Two independent streams share HTTP/2; each stream keeps its requests sequential.
    let catalog_download = async {
        notify(
            "catalog",
            DownloadProgress::new(0, None, "讀取車型目錄".into()),
        );
        let catalog = client
            .call(
                "ListModelCatalogs",
                json!({"meterGenerations":[1,2,3],"onlyOwnCountry":false}),
                token,
            )
            .await?;
        let mut expected = HashMap::new();
        for row in rows(&catalog, "modelCatalogs")? {
            let label = text(&row["model"], "modelSeriesLabel")?.to_string();
            let count = integer(&row["modelCount"])?;
            if count <= 0 || expected.insert(label, count).is_some() {
                return Err("CATALOG_INVALID".into());
            }
        }
        if expected.is_empty() {
            return Err("CATALOG_EMPTY".into());
        }
        let series=client.call("ListModelSeries",json!({"meterGenerations":[1,2,3],"modelSeriesLabels":expected.keys().collect::<Vec<_>>(),"onlyOwnCountry":false}),token).await?;
        let mut models_by_label = HashMap::new();
        let mut series_seen = HashSet::new();
        for group in rows(&series, "modelSeriesList")? {
            let models = rows(group, "models")?;
            let series_label = text(models.first().ok_or("SERIES_EMPTY")?, "modelSeriesLabel")?;
            if expected.get(series_label) != Some(&(models.len() as i64))
                || !series_seen.insert(series_label)
            {
                return Err("SERIES_COUNT_MISMATCH".into());
            }
            for model in models {
                if text(model, "modelSeriesLabel")? != series_label
                    || models_by_label
                        .insert(text(model, "modelLabel")?.to_string(), normalize(model))
                        .is_some()
                {
                    return Err("MODEL_DUPLICATE".into());
                }
            }
        }
        if series_seen.len() != expected.len() {
            return Err("SERIES_COVERAGE_INVALID".into());
        }
        let model_data=client.call("ListModels",json!({"meterGenerations":[1,2,3],"modelLabels":models_by_label.keys().collect::<Vec<_>>(),"countryCode":""}),token).await?;
        let models = rows(&model_data, "models")?;
        if models.len() != models_by_label.len() {
            return Err("MODEL_COUNT_MISMATCH".into());
        }
        let mut seen_models = HashSet::new();
        for model in models {
            let label = text(model, "modelLabel")?;
            if !seen_models.insert(label) || models_by_label.get(label) != Some(&normalize(model)) {
                return Err("MODEL_CONTENT_MISMATCH".into());
            }
        }
        for row in rows(&catalog, "modelCatalogs")? {
            if models_by_label.get(text(&row["model"], "modelLabel")?)
                != Some(&normalize(&row["model"]))
            {
                return Err("CATALOG_CONTENT_MISMATCH".into());
            }
        }
        let image_ids: HashSet<_> = models
            .iter()
            .flat_map(|m| m["images"].as_array().into_iter().flatten())
            .filter_map(|i| i["id"].as_str())
            .collect();
        let mut images = serde_json::Map::new();
        notify(
            "catalog",
            DownloadProgress::new(
                0,
                Some(image_ids.len()),
                format!("車型圖片 · 0 / {} 張", image_ids.len()),
            ),
        );
        for (i, id) in image_ids.iter().enumerate() {
            let image = client
                .call("GetImageBinary", json!({"imageId":id}), token)
                .await?;
            if image["id"] != *id {
                return Err("IMAGE_ID_MISMATCH".into());
            }
            let bytes = STANDARD
                .decode(text(&image, "binary")?)
                .map_err(|_| "IMAGE_INVALID")?;
            let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                "image/png"
            } else if bytes.starts_with(&[255, 216, 255]) {
                "image/jpeg"
            } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
                "image/webp"
            } else {
                return Err("IMAGE_FORMAT_UNSUPPORTED".into());
            };
            images.insert(
                id.to_string(),
                json!(format!("data:{mime};base64,{}", STANDARD.encode(bytes))),
            );
            notify(
                "catalog",
                DownloadProgress::new(
                    i + 1,
                    Some(image_ids.len()),
                    format!("車型圖片 · {} / {} 張", i + 1, image_ids.len()),
                ),
            );
        }
        Ok::<_, String>((models.clone(), expected.len(), images))
    };
    let history_download = async {
        let mut days = Vec::new();
        let mut ids = HashSet::new();
        let mut dates = HashSet::new();
        let mut cursor = chrono::Utc::now().timestamp();
        let mut summary = Value::Null;
        let mut complete = false;
        for page in 0..100 {
            notify(
                "history",
                DownloadProgress::new(0, None, format!("讀取行程清單 · 第 {} 頁", page + 1)),
            );
            let data = client
                .call(
                    "ListDailyTrips",
                    json!({"offsetFromUtc":28800,"startDate":cursor.to_string(),"tripsCount":30}),
                    token,
                )
                .await?;
            if summary.is_null() {
                summary = data["tripsSummary"].clone();
            }
            let batch = rows(&data, "trips")?;
            for day in batch {
                if text(day, "userId")? != uid {
                    return Err("HISTORY_ACCOUNT_MISMATCH".into());
                }
                if !dates.insert(integer(&day["tripDate"])?) {
                    return Err("REPEATED_HISTORY_DAY".into());
                }
                for id in labels(day, "tripUuids")? {
                    if !ids.insert(id) {
                        return Err("REPEATED_TRIP_ID".into());
                    }
                }
                days.push(day.clone());
            }
            if batch.len() < 30 {
                complete = true;
                break;
            }
            let earliest = batch
                .iter()
                .map(|d| integer(&d["startTime"]))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .min()
                .ok_or("PAGE_EMPTY")?;
            let next = (earliest + 28800) / 86400 * 86400 - 28800 - 1;
            if next <= 0 || next >= cursor {
                return Err("PAGINATION_STALLED".into());
            }
            cursor = next;
        }
        if !complete || days.len() as i64 != integer(&summary["totalDriveDay"])? {
            return Err("HISTORY_INCOMPLETE".into());
        }
        let mut details = HashMap::new();
        notify(
            "history",
            DownloadProgress::new(
                0,
                Some(days.len()),
                format!("旅程路線 · 0 / {} 日", days.len()),
            ),
        );
        for (i, day) in days.iter().enumerate() {
            let requested = labels(day, "tripUuids")?;
            if requested.is_empty() {
                notify(
                    "history",
                    DownloadProgress::new(
                        i + 1,
                        Some(days.len()),
                        format!("旅程路線 · {} / {} 日", i + 1, days.len()),
                    ),
                );
                continue;
            }
            if requested.len() > 1000 {
                return Err("DETAIL_LIMIT_EXCEEDED".into());
            }
            let value = client
                .call("ListTripDetails", json!({"tripUuids":requested}), token)
                .await?;
            let batch = rows(&value, "dailyTripDetails")?;
            let returned: HashSet<_> = batch
                .iter()
                .map(|d| text(d, "tripUuid"))
                .collect::<Result<_, _>>()?;
            if returned.len() != batch.len()
                || returned != requested.iter().map(String::as_str).collect()
            {
                return Err("DETAIL_COVERAGE_INVALID".into());
            }
            for detail in batch {
                if details
                    .insert(text(detail, "tripUuid")?.to_string(), detail.clone())
                    .is_some()
                {
                    return Err("DETAIL_DUPLICATE".into());
                }
            }
            notify(
                "history",
                DownloadProgress::new(
                    i + 1,
                    Some(days.len()),
                    format!("旅程路線 · {} / {} 日", i + 1, days.len()),
                ),
            );
        }
        Ok::<_, String>((days, details))
    };
    let ((models, series_count, images), (days, details)) =
        tokio::try_join!(catalog_download, history_download)?;
    let projected = project(&days, &details, &models)?;
    let points: usize = projected
        .iter()
        .flat_map(|d| d["trips"].as_array().into_iter().flatten())
        .map(|t| t["points"].as_array().map_or(0, Vec::len))
        .sum();
    Ok(
        json!({"version":1,"updatedAt":chrono::Utc::now().to_rfc3339(),"accountHash":format!("{:x}",Sha256::digest(uid.as_bytes())),
        "models":models,"seriesCount":series_count,"images":images,"days":projected,
        "summary":{"days":days.len(),"trips":details.len(),"points":points}}),
    )
}
pub fn save(root: &Path, data: &Value) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|_| "CACHE_UNWRITABLE")?;
    // Unique snapshots, then pointer switch: a failed refresh never destroys the last good data.
    let name = format!("snapshot-{}.json", uuid::Uuid::new_v4());
    let bytes = serde_json::to_vec(data).map_err(|_| "CACHE_INVALID")?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    std::fs::write(root.join(&name), bytes).map_err(|_| "CACHE_UNWRITABLE")?;
    let pointer = root.join("latest.json");
    let temporary = root.join("latest.pending");
    std::fs::write(
        &temporary,
        serde_json::to_vec(&json!({"file":name,"sha256":hash})).unwrap(),
    )
    .map_err(|_| "CACHE_UNWRITABLE")?;
    std::fs::rename(&temporary, &pointer).map_err(|_| "CACHE_UNWRITABLE")?;
    Ok(())
}
pub fn load(root: &Path) -> Result<Option<Value>, String> {
    let pointer = root.join("latest.json");
    if !pointer.exists() {
        return Ok(None);
    }
    let value: Value =
        serde_json::from_slice(&std::fs::read(pointer).map_err(|_| "CACHE_UNREADABLE")?)
            .map_err(|_| "CACHE_INVALID")?;
    let file = text(&value, "file")?;
    if !file.starts_with("snapshot-") || !file.ends_with(".json") || file.contains(['/', '\\']) {
        return Err("CACHE_PATH_INVALID".into());
    }
    let bytes = std::fs::read(root.join(file)).map_err(|_| "CACHE_UNREADABLE")?;
    if value["sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err("CACHE_HASH_MISMATCH".into());
    }
    let data: Value = serde_json::from_slice(&bytes).map_err(|_| "CACHE_INVALID")?;
    if data["version"] != 1 {
        return Err("CACHE_VERSION_UNSUPPORTED".into());
    }
    Ok(Some(data))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_switch_keeps_previous_snapshot_and_detects_corruption() {
        let root =
            std::env::temp_dir().join(format!("roadsync-cache-test-{}", uuid::Uuid::new_v4()));
        let first = json!({"version":1,"summary":{"days":1}});
        save(&root, &first).unwrap();
        let previous: Value =
            serde_json::from_slice(&std::fs::read(root.join("latest.json")).unwrap()).unwrap();
        let next = json!({"version":1,"summary":{"days":2}});
        save(&root, &next).unwrap();
        assert_eq!(load(&root).unwrap(), Some(next));
        assert!(root.join(previous["file"].as_str().unwrap()).is_file());
        let latest: Value =
            serde_json::from_slice(&std::fs::read(root.join("latest.json")).unwrap()).unwrap();
        std::fs::write(root.join(latest["file"].as_str().unwrap()), b"corrupted").unwrap();
        assert_eq!(load(&root).unwrap_err(), "CACHE_HASH_MISMATCH");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn match_shared_code_without_inventing_year() {
        let models = vec![
            json!({"modelLabel":"A","modelName":{"displayName":{"fullName":"Bike"}},"vehicleCodes":[{"modelCode":"AB"}]}),
            json!({"modelLabel":"B","modelName":{"displayName":{"fullName":"Other"}},"vehicleCodes":[{"modelCode":"AB"}]}),
        ];
        let day = json!({"vehicleSeriesCodes":["AB"],"vehicleDisplayName":["Bike"]});
        let m = vehicle_match(&day, &models);
        assert_eq!(m["basis"], "code+name");
        assert_eq!(m["models"].as_array().unwrap().len(), 1);
        assert_eq!(m["models"][0]["label"], "A");
        assert_eq!(
            vehicle_match(
                &json!({"vehicleSeriesCodes":["AB"],"vehicleDisplayName":["Unknown"]}),
                &models
            )["nameConflict"],
            true
        );
    }
    #[test]
    fn route_order_and_coverage() {
        let day = json!({"tripUuids":["private-id"],"startTime":"100","endTime":"200","totalDistance":10,"totalTime":"100","avgSpeed":1});
        let detail = json!({"totalDistance":10,"avgSpeed":1,"tripLocations":[{"coordinates":{"lat":1,"lon":2},"timestamp":"200","isCoarse":false},{"coordinates":{"lat":1,"lon":3},"timestamp":"100","isCoarse":false}]});
        let map = HashMap::from([("private-id".to_string(), detail)]);
        let result = project(std::slice::from_ref(&day), &map, &[]).unwrap();
        assert_eq!(result[0]["trips"][0]["start"], 100);
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("private-id"));
        assert!(project(&[day], &HashMap::new(), &[]).is_err());
    }
}
