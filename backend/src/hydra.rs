use rusty_leveldb::{DB, LdbIterator, Options};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;
use std::io::Write;
use reqwest::Client;
use std::collections::HashMap;

use crate::wine::{add_wine_prefix_to_windows_path, get_windows_like_user_profile_path, transform_ludusavi_backup_path_into_windows_path};

struct Snapshot {
    db: DB,
    _temp_dir: TempDir,
}

#[derive(Debug, Deserialize)]
pub struct BackupManifest {
    pub drives: HashMap<String, String>,
    pub backups: Vec<LudusaviBackup>,
}

#[derive(Debug, Deserialize)]
pub struct LudusaviBackup {
    pub files: HashMap<String, FileMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct FileMetadata {
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    remote_id: Option<String>,
    object_id: String,
    shop: String,
    title: String,
    last_time_played: Option<String>,
    play_time_in_milliseconds: f64,
    is_deleted: bool,
    icon_url: Option<String>,
    wine_prefix_path: Option<String>,
    automatic_cloud_sync: Option<bool>,
}

fn get_leveldb_snapshot() -> Snapshot {
    let original_path = dirs::config_dir()
        .unwrap()
        .join("hydralauncher")
        .join("hydra-db");

    let temp_dir = tempfile::tempdir().unwrap();

    fs_extra::dir::copy(
        &original_path,
        temp_dir.path(),
        &fs_extra::dir::CopyOptions {
            content_only: true,
            ..Default::default()
        },
    )
    .unwrap();

    Snapshot {
        db: DB::open(temp_dir.path(), Options::default()).unwrap(),
        _temp_dir: temp_dir,
    }
}

pub fn get_auth() -> String {
    let mut snapshot = get_leveldb_snapshot();

    let auth = match snapshot.db.get(b"auth") {
        Some(auth_data) => String::from_utf8(auth_data).unwrap().to_string(),
        None => String::from(""),
    };

    snapshot.db.close().unwrap();

    auth
}

// Hydra Launcher moved automatic cloud sync to a separate "v2" setting
// (its own sublevel, keyed the same way as the game record) instead of the
// automaticCloudSync field on the game itself. For Steam games, an absent
// v2 setting means enabled by default — see resolveStoredCloudSaveAutomaticSyncMode
// in Hydra Launcher's own source. The legacy field is only meaningful for
// non-Steam shops, which this plugin doesn't otherwise deal with.
const CLOUD_SAVE_AUTOMATIC_SYNC_SETTINGS_PREFIX: &str = "!cloud-save-automatic-sync-settings!";

pub fn get_library() -> String {
    let mut snapshot = get_leveldb_snapshot();

    let mut iter = snapshot.db.new_iter().unwrap();
    let mut games = Vec::new();
    let mut v2_settings: HashMap<String, bool> = HashMap::new();

    while let Some((key_bytes, value_bytes)) = iter.next() {
        let key = String::from_utf8(key_bytes).unwrap();
        if key.starts_with("!games") {
            let game: Game = serde_json::from_str(&String::from_utf8(value_bytes).unwrap()).unwrap();
            games.push(game);
        } else if let Some(suffix) = key.strip_prefix(CLOUD_SAVE_AUTOMATIC_SYNC_SETTINGS_PREFIX) {
            if let Ok(enabled) = serde_json::from_slice::<bool>(&value_bytes) {
                v2_settings.insert(suffix.to_string(), enabled);
            }
        }
    }

    snapshot.db.close().unwrap();

    for game in &mut games {
        let key = format!("{}:{}", game.shop, game.object_id);
        game.automatic_cloud_sync = Some(if game.shop == "steam" {
            v2_settings.get(&key).copied().unwrap_or(true)
        } else {
            game.automatic_cloud_sync.unwrap_or(false)
        });
    }

    serde_json::to_string(&games).unwrap()
}

pub fn toggle_automatic_cloud_sync(shop: &str, object_id: &str, automatic_cloud_sync: bool) -> Result<(), String> {
    let db_path = dirs::config_dir()
        .unwrap()
        .join("hydralauncher")
        .join("hydra-db");

    let key = format!("{}{}:{}", CLOUD_SAVE_AUTOMATIC_SYNC_SETTINGS_PREFIX, shop, object_id);

    let mut db = DB::open(&db_path, Options::default())
        .map_err(|e| format!("Failed to open DB: {:?}", e))?;

    db.put(key.as_bytes(), serde_json::to_string(&automatic_cloud_sync).unwrap().as_bytes())
        .map_err(|e| format!("Failed to write to DB: {:?}", e))?;

    db.close().map_err(|e| format!("Failed to close DB: {:?}", e))?;

    Ok(())
}

fn restore_ludusavi_backup(
    backup_path: PathBuf,
    title: &str,
    home_dir: &str,
    wine_prefix_path: Option<&str>,
    artifact_wine_prefix_path: Option<String>,
) -> std::io::Result<()> {
    let game_backup_path = backup_path.join(title);
    let mapping_yaml_path = game_backup_path.join("mapping.yaml");

    let data = fs::read_to_string(&mapping_yaml_path)?;
    let manifest: BackupManifest = serde_yaml::from_str(&data).unwrap();

    let user_profile_path = get_windows_like_user_profile_path(wine_prefix_path.unwrap()).unwrap();

    for backup in manifest.backups {
        for key in backup.files.keys() {
            let mut source_path_with_drives = key.clone();

            for (drive_key, drive_value) in &manifest.drives {
                source_path_with_drives = source_path_with_drives.replacen(drive_value, drive_key, 1);
            }

            let source_path = game_backup_path.join(&source_path_with_drives);

            let public_path = "C:/users/Public";

            let destination_path = transform_ludusavi_backup_path_into_windows_path(key, artifact_wine_prefix_path.clone())
                .replacen(
                    home_dir,
                    &add_wine_prefix_to_windows_path(&user_profile_path, wine_prefix_path),
                    1,
                )
                .replacen(
                    &public_path,
                    &add_wine_prefix_to_windows_path(&public_path, wine_prefix_path),
                    1,
                );

            let destination_path = PathBuf::from(destination_path);

            println!("Moving {} to {}", source_path.display(), destination_path.display());

            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }

            if destination_path.exists() {
                fs::remove_file(&destination_path)?;
            }

            fs::rename(source_path, destination_path)?;
        }
    }

    Ok(())
}

pub async fn download_game_artifact(
    object_id: &str,
    shop: &str,
    download_url: &str,
    object_key: &str,
    home_dir: &str,
    wine_prefix_path: Option<&str>,
    artifact_wine_prefix_path: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let backups_path = dirs::config_dir()
        .unwrap()
        .join("hydralauncher")
        .join("Backups");

    fs::create_dir_all(&backups_path)?;

    let zip_location = backups_path.join(object_key);
    let backup_path = backups_path.join(format!("{}-{}", shop, object_id));

    if backup_path.exists() {
        fs::remove_dir_all(&backup_path)?;
    }

    let client = Client::new();
    let mut response = client.get(download_url).send().await?;

    let mut file = File::create(&zip_location)?;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk)?;
    }

    fs::create_dir_all(&backup_path)?;

    let archive_file = File::open(&zip_location)?;
    let mut archive = Archive::new(archive_file);
    archive.unpack(&backup_path)?;

    restore_ludusavi_backup(
        backup_path,
        object_id,
        home_dir,
        wine_prefix_path,
        artifact_wine_prefix_path,
    )?;

    Ok(())
}