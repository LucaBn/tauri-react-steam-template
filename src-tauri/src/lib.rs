use serde::Serialize;
use std::sync::Mutex;
use steamworks::Client;
use tauri::{command, Emitter, State};

const STEAM_APP_ID: u32 = 480; // TODO: change with your Steam App ID

struct SteamState {
    client: Mutex<Option<Client>>,
}

#[derive(Serialize, Clone)]
pub struct LeaderboardEntryDto {
    pub rank: i32,
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
    F: FnOnce(&steamworks::Client) -> Result<T, String>,
{
    let client_guard = state.client.lock().map_err(|e| e.to_string())?;

    let client = client_guard
        .as_ref()
        .ok_or_else(|| "Steamworks client not initialized".to_string())?;

    f(client)
}

fn find_leaderboard(
    client: &steamworks::Client,
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
    client: &steamworks::Client,
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
    client: &steamworks::Client,
    steam_id: steamworks::SteamId,
) -> String {
    let friends = client.friends();

    friends.request_user_information(steam_id, true);

    let name = friends.get_friend(steam_id).name();

    if name.is_empty() || name == "[unknown]" {
        let steam_id_string = steam_id.raw().to_string();

        if steam_id_string.len() >= 6 {
            format!("Player_{}", &steam_id_string[..6])
        } else {
            format!("Player_{}", steam_id_string)
        }
    } else {
        name
    }
}

fn map_entries_to_dtos(
    client: &steamworks::Client,
    entries: Vec<steamworks::LeaderboardEntry>,
) -> Vec<LeaderboardEntryDto> {
    let current_user_id = client.user().steam_id();

    entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let steam_name = get_safe_steam_name(client, entry.user);

            LeaderboardEntryDto {
                rank: (index + 1) as i32,
                global_rank: entry.global_rank,
                score: entry.score,
                steam_name,
                is_current_user: entry.user == current_user_id,
            }
        })
        .collect()
}

fn fetch_leaderboard_logic(
    client: &steamworks::Client,
    leaderboard_name: &str,
    request_type: steamworks::LeaderboardDataRequest,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    let leaderboard = find_leaderboard(client, leaderboard_name)?;
    let current_user_id = client.user().steam_id();

    match request_type {
        steamworks::LeaderboardDataRequest::Friends => {
            let raw_entries = download_entries(
                client,
                &leaderboard,
                steamworks::LeaderboardDataRequest::Friends,
                0,
                0,
            )?;

            let user_index = raw_entries
                .iter()
                .position(|entry| entry.user == current_user_id);

            let mut result_list = raw_entries
                .iter()
                .take(10)
                .enumerate()
                .map(|(index, entry)| {
                    let steam_name = get_safe_steam_name(client, entry.user);

                    LeaderboardEntryDto {
                        rank: (index + 1) as i32,
                        global_rank: entry.global_rank,
                        score: entry.score,
                        steam_name,
                        is_current_user: entry.user == current_user_id,
                    }
                })
                .collect::<Vec<_>>();

            if let Some(index) = user_index {
                if index >= 10 {
                    let user_entry = &raw_entries[index];

                    let steam_name = get_safe_steam_name(client, user_entry.user);

                    result_list.push(LeaderboardEntryDto {
                        rank: (index + 1) as i32,
                        global_rank: user_entry.global_rank,
                        score: user_entry.score,
                        steam_name,
                        is_current_user: true,
                    });
                }
            }

            Ok(result_list)
        }

        steamworks::LeaderboardDataRequest::Global => {
            let raw_entries = download_entries(
                client,
                &leaderboard,
                steamworks::LeaderboardDataRequest::Global,
                1,
                10,
            )?;

            let user_in_top_10 = raw_entries
                .iter()
                .any(|entry| entry.user == current_user_id);

            let mut result_list = map_entries_to_dtos(client, raw_entries);

            if !user_in_top_10 {
                if let Ok(user_raw_entries) = download_entries(
                    client,
                    &leaderboard,
                    steamworks::LeaderboardDataRequest::GlobalAroundUser,
                    0,
                    0,
                ) {
                    if let Some(user_entry) = user_raw_entries
                        .into_iter()
                        .find(|entry| entry.user == current_user_id)
                    {
                        let steam_name =
                            get_safe_steam_name(client, user_entry.user);

                        result_list.push(LeaderboardEntryDto {
                            rank: user_entry.global_rank,
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

        _ => Err("Unsupported leaderboard request type".to_string()),
    }
}

// ==========================================
// TAURI COMMANDS
// ==========================================

/// Keeps compatibility with the existing frontend.
///
/// Steam is initialized before the Tauri application starts,
/// so this command no longer performs the initialization itself.
#[tauri::command]
fn steam_init(state: State<'_, SteamState>) -> Result<String, String> {
    let client_guard = state.client.lock().map_err(|e| e.to_string())?;

    if client_guard.is_some() {
        return Ok("Steamworks already initialized".into());
    }

    Err("Steamworks client not initialized".into())
}

#[tauri::command]
fn get_current_username(state: State<'_, SteamState>) -> Result<String, String> {
    with_client(&state, |client| {
        let steam_id = client.user().steam_id();

        Ok(get_safe_steam_name(client, steam_id))
    })
}

#[tauri::command]
fn upload_score(
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
fn unlock_achievement(
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

#[command]
async fn get_most_wins_leaderboard(
    state: State<'_, SteamState>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    with_client(&state, |client| {
        fetch_leaderboard_logic(
            client,
            "most_wins",
            steamworks::LeaderboardDataRequest::Global,
        )
    })
}

#[command]
async fn get_friends_wins_leaderboard(
    state: State<'_, SteamState>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    with_client(&state, |client| {
        fetch_leaderboard_logic(
            client,
            "most_wins",
            steamworks::LeaderboardDataRequest::Friends,
        )
    })
}

#[command]
async fn get_best_time_leaderboard(
    state: State<'_, SteamState>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    with_client(&state, |client| {
        fetch_leaderboard_logic(
            client,
            "best_time",
            steamworks::LeaderboardDataRequest::Global,
        )
    })
}

#[command]
async fn get_friends_best_time_leaderboard(
    state: State<'_, SteamState>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    with_client(&state, |client| {
        fetch_leaderboard_logic(
            client,
            "best_time",
            steamworks::LeaderboardDataRequest::Friends,
        )
    })
}

#[tauri::command]
fn activate_overlay(state: State<'_, SteamState>) -> Result<(), String> {
    with_client(&state, |client| {
        client.friends().activate_game_overlay("");

        Ok(())
    })
}

#[tauri::command]
fn trigger_screenshot(
    app: tauri::AppHandle,
    state: State<'_, SteamState>,
) -> Result<(), String> {
    let client_guard = state.client.lock().map_err(|e| e.to_string())?;

    let client = client_guard
        .as_ref()
        .ok_or_else(|| "Steamworks client not initialized".to_string())?;

    let shot =
        tauri_plugin_steam_overlay_surface::capture_screenshot_png(&app)
            .ok_or_else(|| "Failed to capture screenshot".to_string())?;

    client
        .screenshots()
        .add_screenshot_to_library(
            &shot.path,
            None,
            shot.width as i32,
            shot.height as i32,
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ==========================================
// TAURI APPLICATION
// ==========================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ==========================================
    // STEAM INITIALIZATION
    // ==========================================
    //
    // Steam must be initialized BEFORE creating the Tauri
    // application because the Steam overlay surface plugin
    // needs Steam to be initialized before creating its
    // graphics surface.
    //

    let steam_client = match Client::init_app(STEAM_APP_ID) {
    Ok(client) => {
            let pump = client.clone();

            std::thread::spawn(move || {
                loop {
                    pump.run_callbacks();
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            });

            Some(client)
        }
        Err(error) => {
            eprintln!("Failed to initialize Steamworks: {error}");
            None
        }
    };

    // Clone the Steam client for the callbacks.
    let steam_callback_client = steam_client.clone();

    // ==========================================
    // TAURI BUILDER
    // ==========================================

    tauri::Builder::default()

        // ==========================================
        // STEAM OVERLAY SURFACE
        // ==========================================

        .plugin(
            tauri_plugin_steam_overlay_surface::Builder::new()
                .main_window_label("main")
                .overlay_title("Steam App name") // TODO: change with your Steam App name
                .snapshot_backdrop(true)
                .build(),
        )

        // ==========================================
        // EXISTING PLUGINS
        // ==========================================

        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())

        // ==========================================
        // TAURI STATE
        // ==========================================

        .manage(SteamState {
            client: Mutex::new(steam_client),
        })

        // ==========================================
        // COMMANDS
        // ==========================================

        .invoke_handler(tauri::generate_handler![
            steam_init,
            get_current_username,
            upload_score,
            get_most_wins_leaderboard,
            get_friends_wins_leaderboard,
            get_best_time_leaderboard,
            get_friends_best_time_leaderboard,
            unlock_achievement,
            activate_overlay,
            trigger_screenshot
        ])

        // ==========================================
        // WINDOW EVENTS
        // ==========================================

        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                std::process::exit(0);
            }
        })

        // ==========================================
        // SETUP
        // ==========================================

        .setup(move |app| {
            if let Some(client) = steam_callback_client.as_ref() {
                // ------------------------------------------
                // Steam overlay activation
                // ------------------------------------------

                let handle = app.handle().clone();

                let overlay_callback = client.register_callback(
                    move |event: steamworks::GameOverlayActivated| {
                        tauri_plugin_steam_overlay_surface::on_overlay_activated(
                            &handle,
                            event.active,
                        );
                    },
                );

                // Keep the callback alive for the application lifetime.
                std::mem::forget(overlay_callback);

                // ------------------------------------------
                // Steam screenshots (F12)
                // ------------------------------------------

                client.screenshots().hook_screenshots(true);

                let handle = app.handle().clone();

                let callback = client.register_callback(
                    move |_event: steamworks::screenshots::ScreenshotRequested| {
                        if let Some(screenshot) =
                            tauri_plugin_steam_overlay_surface::capture_screenshot_png(&handle)
                        {
                            let _ = handle.emit(
                                "steam-screenshot-captured",
                                screenshot.path.to_string_lossy().to_string(),
                            );
                        }
                    },
                );

                std::mem::forget(callback);
            }

            // ------------------------------------------
            // Debug logging
            // ------------------------------------------

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            Ok(())
        })

        // ==========================================
        // RUN
        // ==========================================

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}