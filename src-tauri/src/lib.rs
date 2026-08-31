use serde::Serialize;
use std::sync::Mutex;
use steamworks::Client;
use tauri::State;

// ==========================================
// STATE & STRUCTS
// ==========================================

pub struct SteamState {
    pub client: Mutex<Option<Client>>,
}

impl Default for SteamState {
    fn default() -> Self {
        Self {
            client: Mutex::new(None),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct LeaderboardEntryDto {
    pub global_rank: i32,
    pub score: i32,
    pub steam_name: String,
    pub is_current_user: bool,
}

// ==========================================
// HELPER FUNCTIONS
// ==========================================

fn with_client<F, T>(state: &SteamState, f: F) -> Result<T, String>
where
    F: FnOnce(&steamworks::Client<steamworks::ClientManager>) -> Result<T, String>,
{
    let client_guard = state.client.lock().map_err(|e| e.to_string())?;
    let client = client_guard
        .as_ref()
        .ok_or_else(|| "Steamworks client not initialized".to_string())?;
    f(client)
}

fn find_leaderboard(
    client: &steamworks::Client<steamworks::ClientManager>,
    name: &str,
) -> Result<steamworks::Leaderboard, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let name_string = name.to_string();

    client
        .user_stats()
        .find_leaderboard(&name_string, move |result| {
            let _ = tx.send(result);
        });

    rx.recv()
        .map_err(|_| "Failed to receive leaderboard response".to_string())?
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Leaderboard '{}' not found", name))
}

fn download_entries(
    client: &steamworks::Client<steamworks::ClientManager>,
    leaderboard: &steamworks::Leaderboard,
    request_type: steamworks::LeaderboardDataRequest,
    start: usize,
    end: usize,
) -> Result<Vec<steamworks::LeaderboardEntry>, String> {
    let (tx, rx) = std::sync::mpsc::channel();

    client.user_stats().download_leaderboard_entries(
        leaderboard,
        request_type,
        start,
        end,
        0,
        move |result| {
            let _ = tx.send(result);
        },
    );

    rx.recv()
        .map_err(|_| "Failed to receive leaderboard entries".to_string())?
        .map_err(|e| e.to_string())
}

fn get_safe_steam_name(
    client: &steamworks::Client<steamworks::ClientManager>,
    steam_id: steamworks::SteamId,
) -> String {
    let friends = client.friends();
    friends.request_user_information(steam_id, true);

    let name = friends.get_friend(steam_id).name();
    if name.is_empty() || name == "[unknown]" {
        format!("Player_{}", &steam_id.raw().to_string()[..6])
    } else {
        name
    }
}

fn map_entries_to_dtos(
    client: &steamworks::Client<steamworks::ClientManager>,
    entries: Vec<steamworks::LeaderboardEntry>,
) -> Vec<LeaderboardEntryDto> {
    let current_user_id = client.user().steam_id();
    entries
        .into_iter()
        .map(|entry| {
            let steam_name = get_safe_steam_name(client, entry.user);
            LeaderboardEntryDto {
                global_rank: entry.global_rank,
                score: entry.score,
                steam_name,
                is_current_user: entry.user == current_user_id,
            }
        })
        .collect()
}

fn fetch_leaderboard_logic(
    client: &steamworks::Client<steamworks::ClientManager>,
    leaderboard_name: &str,
    request_type: steamworks::LeaderboardDataRequest,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    let leaderboard = find_leaderboard(client, leaderboard_name)?;

    if let steamworks::LeaderboardDataRequest::Friends = request_type {
        let raw_entries = download_entries(client, &leaderboard, request_type, 1, 10)?;
        return Ok(map_entries_to_dtos(client, raw_entries));
    }

    let raw_entries = download_entries(
        client,
        &leaderboard,
        steamworks::LeaderboardDataRequest::Global,
        1,
        10,
    )?;
    let mut result_list = map_entries_to_dtos(client, raw_entries);

    let user_in_top_10 = result_list.iter().any(|d| d.is_current_user);
    if !user_in_top_10 {
        if let Ok(user_raw_entries) = download_entries(
            client,
            &leaderboard,
            steamworks::LeaderboardDataRequest::GlobalAroundUser,
            0,
            0,
        ) {
            let current_user_id = client.user().steam_id();
            if let Some(user_entry) = user_raw_entries
                .into_iter()
                .find(|e| e.user == current_user_id)
            {
                let steam_name = get_safe_steam_name(client, user_entry.user);
                result_list.push(LeaderboardEntryDto {
                    global_rank: user_entry.global_rank,
                    score: user_entry.score,
                    steam_name,
                    is_current_user: true,
                });
            }
        }
    }

    Ok(result_list)
}

// ==========================================
// TAURI COMMANDS MODULE (Isolated scope)
// ==========================================

pub mod commands {
    use super::*;

    #[tauri::command]
    pub fn steam_init(app_id: u32, state: State<'_, SteamState>) -> Result<String, String> {
        let mut client_guard = state.client.lock().map_err(|e| e.to_string())?;

        if client_guard.is_some() {
            return Ok("Steamworks already initialized".into());
        }

        let (client, single) = Client::init_app(app_id).map_err(|e| e.to_string())?;

        std::thread::spawn(move || loop {
            single.run_callbacks();
            std::thread::sleep(std::time::Duration::from_millis(100));
        });

        *client_guard = Some(client);
        Ok("Steamworks initialized successfully".into())
    }

    #[tauri::command]
    pub fn get_current_username(state: State<'_, SteamState>) -> Result<String, String> {
        with_client(&state, |client| {
            let steam_id = client.user().steam_id();
            Ok(get_safe_steam_name(client, steam_id))
        })
    }

    #[tauri::command]
    pub fn upload_score(
        leaderboard_name: String,
        score: i32,
        state: State<'_, SteamState>,
    ) -> Result<String, String> {
        with_client(&state, |client| {
            let leaderboard = find_leaderboard(client, &leaderboard_name)?;
            let (upload_tx, upload_rx) = std::sync::mpsc::channel();

            client.user_stats().upload_leaderboard_score(
                &leaderboard,
                steamworks::UploadScoreMethod::KeepBest,
                score,
                &[],
                move |result| {
                    let _ = upload_tx.send(result);
                },
            );

            let upload_result = upload_rx
                .recv()
                .map_err(|_| "Failed to receive upload score response".to_string())?
                .map_err(|e| e.to_string())?;

            match upload_result {
                Some(details) => Ok(format!(
                    "Score uploaded successfully. Score: {}, Score Changed: {}",
                    details.score, details.was_changed
                )),
                None => Ok(format!(
                    "Score processing complete for '{}'. Score was not updated.",
                    leaderboard_name
                )),
            }
        })
    }

    #[tauri::command]
    pub fn unlock_achievement(
        achievement_id: String,
        state: State<'_, SteamState>,
    ) -> Result<String, String> {
        with_client(&state, |client| {
            let stats = client.user_stats();
            stats.achievement(&achievement_id).set();

            match stats.store_stats() {
                Ok(()) => Ok(format!(
                    "Achievement '{}' unlocked successfully",
                    achievement_id
                )),
                Err(()) => Err("Failed to store stats on Steam".into()),
            }
        })
    }

    #[tauri::command]
    pub async fn get_global_leaderboard(
        leaderboard_name: String,
        state: State<'_, SteamState>,
    ) -> Result<Vec<LeaderboardEntryDto>, String> {
        with_client(&state, |client| {
            fetch_leaderboard_logic(
                client,
                &leaderboard_name,
                steamworks::LeaderboardDataRequest::Global,
            )
        })
    }

    #[tauri::command]
    pub async fn get_friends_leaderboard(
        leaderboard_name: String,
        state: State<'_, SteamState>,
    ) -> Result<Vec<LeaderboardEntryDto>, String> {
        with_client(&state, |client| {
            fetch_leaderboard_logic(
                client,
                &leaderboard_name,
                steamworks::LeaderboardDataRequest::Friends,
            )
        })
    }
}

// ==========================================
// TAURI ENTRYPOINT
// ==========================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SteamState::default())
        .invoke_handler(tauri::generate_handler![
            commands::steam_init,
            commands::get_current_username,
            commands::upload_score,
            commands::get_global_leaderboard,
            commands::get_friends_leaderboard,
            commands::unlock_achievement
        ])
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}