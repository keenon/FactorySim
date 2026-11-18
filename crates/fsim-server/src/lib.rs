//! `fsim-server`: Game and MCTS server binary + library.
//!
//! Features:
//! - Invite-only user accounts (`data/users.sqlite`) with root admin bootstrap, single-use invite links,
//!   admin-generated password reset links, session cookies, and roles (`player`, `operations`, `admin`).
//! - Disk storage (`data/games/<game-id>/`) with `game.json`, zstd-compressed hash-chained `.log.zst`
//!   segments, `.snap.zst` estimated-state and truth snapshots, and `checkpoints.json`.
//! - Actor-per-game state manager with WebSocket live streaming, play/pause/speed, skip-ahead with
//!   runway summary card, checkpoints/rewind, planning-mode validation, and "Report a problem" bundles.

pub mod reference;
pub mod sqlite;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path as AxumPath, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use fsim_core::{CellCatalogEntry, CellModel, CellStatusReport, SceneNode, SimTime};
use fsim_facilities_ehs::{starter_building_listings, BuildingListing};
use fsim_finance::RunwayProjection;
use fsim_gateway::GatewayState;
use fsim_planning_flow::BottleneckReport;
use fsim_plant_host::{catalog_preview_model, full_starter_catalog, SimWorld, SkipSummaryCard};
use fsim_protocols::PlantCommand;
use fsim_sales::{
    starter_certifications, starter_markets, CertificationSpec, MarketDefinition,
};
use serde::{Deserialize, Serialize};
use sqlite::SqliteConn;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;
use uuid::Uuid;

/// User account record stored in `users.sqlite`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: String,
    pub email: String,
    pub role: String, // "player", "operations", "admin"
}

/// Summary of a saved game on the Game Setup Screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummaryItem {
    pub game_id: String,
    pub owner_id: String,
    pub scenario_name: String,
    pub game_seed: u64,
    pub sim_time_ms: i64,
    pub cash_tenth_cents: i64,
    pub runway_days_p50: i64,
    pub plant_count: usize,
    pub bankrupt: bool,
}

/// Named or autosave checkpoint metadata in `checkpoints.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub checkpoint_id: String,
    pub name: String,
    pub is_autosave: bool,
    pub sim_time_ms: i64,
    pub log_seq: u64,
    pub cash_tenth_cents: i64,
    pub runway_days_p50: i64,
}

/// Full client-facing view of a game's estimated state, derived dashboards, and status reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientGameSnapshot {
    pub world: SimWorld,
    pub cell_statuses: Vec<CellStatusReport>,
    pub scene_nodes_by_plant: BTreeMap<String, Vec<SceneNode>>,
    pub runway: RunwayProjection,
    pub bottleneck: BottleneckReport,
    pub checkpoints: Vec<CheckpointMeta>,
    pub last_skip_summary: Option<SkipSummaryCard>,
}

/// Static catalog bundle sent once and cached by the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticCatalogBundle {
    pub buildings: Vec<BuildingListing>,
    pub cells: Vec<CellCatalogEntry>,
    pub markets: Vec<MarketDefinition>,
    pub certifications: Vec<CertificationSpec>,
    /// Team-generated 3D preview models keyed by `catalog_id`, for planning-mode ghosts.
    pub preview_models: BTreeMap<String, CellModel>,
}

/// Active game actor held in server memory and persisted to `data/games/<game-id>/`.
pub struct GameInstance {
    pub owner_id: String,
    pub world: SimWorld,
    pub checkpoints: Vec<CheckpointMeta>,
    pub checkpoint_states: BTreeMap<String, SimWorld>,
    pub last_skip_summary: Option<SkipSummaryCard>,
    pub speed_multiplier: u32, // 0 = paused, 1 = 1x, 5 = 5x, 20 = 20x
}

impl GameInstance {
    pub fn to_client_snapshot(&self) -> ClientGameSnapshot {
        let mut scenes = BTreeMap::new();
        for pid in self.world.plants.keys() {
            scenes.insert(pid.clone(), self.world.visualize_plant(pid));
        }
        ClientGameSnapshot {
            world: self.world.clone(),
            cell_statuses: self.world.cell_status_reports(),
            scene_nodes_by_plant: scenes,
            runway: self.world.runway_projection(),
            bottleneck: self.world.bottleneck_report(),
            checkpoints: self.checkpoints.clone(),
            last_skip_summary: self.last_skip_summary.clone(),
        }
    }
}

/// Shared server state across HTTP and WebSocket handlers.
#[derive(Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub db: Arc<Mutex<SqliteConn>>,
    pub games: Arc<Mutex<BTreeMap<String, GameInstance>>>,
}

impl AppState {
    /// Initializes `data/users.sqlite`, bootstraps a default root admin account if none exists,
    /// and loads existing games from `data/games/`.
    pub fn init(data_dir: &Path) -> Result<(Self, Option<String>), String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("users.sqlite");
        let conn = SqliteConn::open(&db_path)?;

        conn.exec(
            "CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                salt_hex TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                failed_attempts INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS invites (
                token TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS password_resets (
                token TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                email TEXT NOT NULL,
                role TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                actor TEXT NOT NULL,
                action TEXT NOT NULL
            );",
        )?;

        // Bootstrap root admin if no users exist yet
        let rows = conn.query_rows("SELECT COUNT(*) FROM users;", &[])?;
        let count: usize = rows
            .first()
            .and_then(|r| r.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut bootstrap_invite = None;
        if count == 0 {
            let salt = "root-admin-salt-01";
            let hash = reference::reference_hash_password("admin123", salt);
            conn.execute_params(
                "INSERT INTO users (user_id, email, salt_hex, password_hash, role) VALUES (?1, ?2, ?3, ?4, ?5);",
                &["usr-admin", "admin@factory.local", salt, &hash, "admin"],
            )?;
            let inv_token = "invite-founder-001";
            conn.execute_params(
                "INSERT INTO invites (token, role, used) VALUES (?1, ?2, 0);",
                &[inv_token, "player"],
            )?;
            bootstrap_invite = Some(inv_token.to_string());
        }

        let state = Self {
            data_dir: data_dir.to_path_buf(),
            db: Arc::new(Mutex::new(conn)),
            games: Arc::new(Mutex::new(BTreeMap::new())),
        };
        state.load_games_from_disk();
        Ok((state, bootstrap_invite))
    }

    fn load_games_from_disk(&self) {
        let games_dir = self.data_dir.join("games");
        let Ok(entries) = std::fs::read_dir(&games_dir) else {
            return;
        };
        let mut games = self.games.lock().unwrap();
        for entry in entries.flatten() {
            let dir = entry.path();
            let snap_path = dir.join("snapshots").join("latest.snap.zst");
            let meta_path = dir.join("game.json");
            if let (Ok(compressed), Ok(meta_bytes)) =
                (std::fs::read(&snap_path), std::fs::read(&meta_path))
            {
                if let (Ok(raw), Ok(meta_val)) = (
                    zstd::decode_all(&compressed[..]),
                    serde_json::from_slice::<serde_json::Value>(&meta_bytes),
                ) {
                    if let Ok(world) = serde_json::from_slice::<SimWorld>(&raw) {
                        let owner_id = meta_val
                            .get("owner_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("usr-admin")
                            .to_string();
                        let ck_path = dir.join("checkpoints.json");
                        let checkpoints: Vec<CheckpointMeta> = std::fs::read(&ck_path)
                            .ok()
                            .and_then(|b| serde_json::from_slice(&b).ok())
                            .unwrap_or_default();
                        let mut ck_states = BTreeMap::new();
                        for ck in &checkpoints {
                            ck_states.insert(ck.checkpoint_id.clone(), world.clone());
                        }
                        games.insert(
                            world.game_id.clone(),
                            GameInstance {
                                owner_id,
                                world,
                                checkpoints,
                                checkpoint_states: ck_states,
                                last_skip_summary: None,
                                speed_multiplier: 0,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Safely persists `GameInstance` to `data/games/<game-id>/` using atomic temp-file renames
    /// and zstd-compressed segments/snapshots per ROUGH_PLAN.md.
    pub fn persist_game(&self, inst: &GameInstance) -> Result<(), String> {
        let game_dir = self.data_dir.join("games").join(&inst.world.game_id);
        let log_dir = game_dir.join("log");
        let snap_dir = game_dir.join("snapshots");
        let truth_dir = game_dir.join("truth");
        std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&snap_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&truth_dir).map_err(|e| e.to_string())?;

        let game_meta = serde_json::json!({
            "game_id": inst.world.game_id,
            "owner_id": inst.owner_id,
            "scenario": inst.world.scenario_name,
            "seed": inst.world.game_seed,
            "schema_version": 1,
        });
        reference::atomic_write_file(
            &game_dir.join("game.json"),
            &serde_json::to_vec_pretty(&game_meta).map_err(|e| e.to_string())?,
        )?;

        // Write hash-chained message log segment (`00001.log.zst`)
        let zst_log = GatewayState::compress_segment_zst(&inst.world.gateway.log_entries)?;
        reference::atomic_write_file(&log_dir.join("00001.log.zst"), &zst_log)?;

        // Write zstd-compressed estimated-state and truth snapshots
        let world_bytes = serde_json::to_vec(&inst.world).map_err(|e| e.to_string())?;
        let world_zst = zstd::encode_all(&world_bytes[..], 3).map_err(|e| e.to_string())?;
        let seq = inst.world.gateway.log_entries.len();
        reference::atomic_write_file(&snap_dir.join(format!("{seq:06}.snap.zst")), &world_zst)?;
        reference::atomic_write_file(&snap_dir.join("latest.snap.zst"), &world_zst)?;
        reference::atomic_write_file(&truth_dir.join(format!("{seq:06}.snap.zst")), &world_zst)?;

        // Write `checkpoints.json`
        let ck_bytes = serde_json::to_vec_pretty(&inst.checkpoints).map_err(|e| e.to_string())?;
        reference::atomic_write_file(&game_dir.join("checkpoints.json"), &ck_bytes)?;
        Ok(())
    }

    pub fn authenticate_headers(&self, headers: &HeaderMap) -> Option<UserAccount> {
        let mut token = None;
        if let Some(auth) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
            if let Some(stripped) = auth.strip_prefix("Bearer ") {
                token = Some(stripped.trim().to_string());
            }
        }
        if token.is_none() {
            if let Some(cookie_str) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
                for part in cookie_str.split(';') {
                    let trimmed = part.trim();
                    if let Some(val) = trimmed.strip_prefix("fsim_session=") {
                        token = Some(val.to_string());
                    }
                }
            }
        }
        let session_id = token?;
        let db = self.db.lock().unwrap();
        let rows = db
            .query_rows(
                "SELECT user_id, email, role FROM sessions WHERE session_id = ?1;",
                &[&session_id],
            )
            .ok()?;
        let row = rows.first()?;
        Some(UserAccount {
            user_id: row[0].clone(),
            email: row[1].clone(),
            role: row[2].clone(),
        })
    }
}

// --- HTTP / JSON API Request & Response Types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub session_token: String,
    pub user: UserAccount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterInviteRequest {
    pub invite_token: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInviteRequest {
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateResetRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UseResetRequest {
    pub reset_token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub scenario_name: String,
    pub game_seed: u64,
    pub auto_pause_on_alerts: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatePlacementRequest {
    pub plant_id: String,
    pub catalog_id: String,
    pub x_mm: i32,
    pub y_mm: i32,
    pub rotation_deg: i32,
    pub ignore_cell_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatePlacementResponse {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SkipAheadRequest {
    pub max_horizon_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveCheckpointRequest {
    pub name: String,
    pub is_autosave: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RewindCheckpointRequest {
    pub checkpoint_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProblemReportRequest {
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProblemReportBundle {
    pub report_id: String,
    pub game_id: String,
    pub game_seed: u64,
    pub sim_time_ms: i64,
    pub log_seq: u64,
    pub latest_checkpoint_id: Option<String>,
    pub note: String,
}

/// Builds the complete Axum router for `fsim-server`.
pub fn build_router(state: AppState, web_dist_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/api/auth/login", post(api_login))
        .route("/api/auth/logout", post(api_logout))
        .route("/api/auth/me", get(api_me))
        .route("/api/auth/register", post(api_register_with_invite))
        .route("/api/auth/invite", post(api_create_invite))
        .route("/api/auth/reset-link", post(api_create_reset_link))
        .route("/api/auth/reset-password", post(api_use_reset_link))
        .route("/api/catalogs", get(api_catalogs))
        .route("/api/games", get(api_list_games).post(api_create_game))
        .route("/api/games/:game_id", get(api_get_game))
        .route("/api/games/:game_id/command", post(api_apply_command))
        .route("/api/games/:game_id/skip", post(api_skip_ahead))
        .route("/api/games/:game_id/step", post(api_step_small))
        .route("/api/games/:game_id/validate-placement", post(api_validate_placement))
        .route("/api/games/:game_id/checkpoint", post(api_save_checkpoint))
        .route("/api/games/:game_id/rewind", post(api_rewind_checkpoint))
        .route("/api/games/:game_id/report", post(api_report_problem))
        .route("/ws/games/:game_id", get(ws_game_handler))
        .with_state(state);

    if let Some(dir) = web_dist_dir {
        api.fallback_service(ServeDir::new(dir))
    } else {
        api
    }
}

fn unauthorized(msg: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": msg })),
    )
        .into_response()
}

fn bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": msg })),
    )
        .into_response()
}

async fn api_login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Response {
    let db = state.db.lock().unwrap();
    let Ok(rows) = db.query_rows(
        "SELECT user_id, email, salt_hex, password_hash, role, failed_attempts FROM users WHERE email = ?1;",
        &[&req.email],
    ) else {
        return unauthorized("Invalid credentials");
    };
    let Some(row) = rows.first() else {
        return unauthorized("Invalid credentials");
    };
    let failed: i32 = row[5].parse().unwrap_or(0);
    if failed >= 5 {
        return unauthorized("Account temporarily locked after repeated failed sign-in attempts");
    }

    if !reference::reference_verify_password(&req.password, &row[2], &row[3]) {
        let new_failed = (failed + 1).to_string();
        let _ = db.execute_params(
            "UPDATE users SET failed_attempts = ?1 WHERE email = ?2;",
            &[&new_failed, &req.email],
        );
        return unauthorized("Invalid email or password");
    }

    let _ = db.execute_params(
        "UPDATE users SET failed_attempts = 0 WHERE email = ?1;",
        &[&req.email],
    );

    let session_token = format!("sess-{}", Uuid::new_v4());
    let user = UserAccount {
        user_id: row[0].clone(),
        email: row[1].clone(),
        role: row[4].clone(),
    };
    let _ = db.execute_params(
        "INSERT INTO sessions (session_id, user_id, email, role) VALUES (?1, ?2, ?3, ?4);",
        &[&session_token, &user.user_id, &user.email, &user.role],
    );
    let _ = db.execute_params(
        "INSERT INTO audit_log (actor, action) VALUES (?1, ?2);",
        &[&user.email, "sign_in"],
    );

    let cookie = format!(
        "fsim_session={session_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800"
    );
    let mut resp = Json(AuthResponse {
        session_token,
        user,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

async fn api_logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(u) = state.authenticate_headers(&headers) {
        let db = state.db.lock().unwrap();
        let _ = db.execute_params("DELETE FROM sessions WHERE user_id = ?1;", &[&u.user_id]);
    }
    let mut resp = Json(serde_json::json!({ "ok": true })).into_response();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        "fsim_session=; Path=/; HttpOnly; Max-Age=0".parse().unwrap(),
    );
    resp
}

async fn api_me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(user) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    Json(user).into_response()
}

async fn api_register_with_invite(
    State(state): State<AppState>,
    Json(req): Json<RegisterInviteRequest>,
) -> Response {
    let db = state.db.lock().unwrap();
    let Ok(rows) = db.query_rows(
        "SELECT role, used FROM invites WHERE token = ?1;",
        &[&req.invite_token],
    ) else {
        return bad_request("Invalid invite link");
    };
    let Some(row) = rows.first() else {
        return bad_request("Invalid invite link");
    };
    if row[1] != "0" {
        return bad_request("Invite link has already been used");
    }

    let role = row[0].clone();
    let user_id = format!("usr-{}", Uuid::new_v4());
    let salt = format!("salt-{}", Uuid::new_v4());
    let hash = reference::reference_hash_password(&req.password, &salt);

    if let Err(e) = db.execute_params(
        "INSERT INTO users (user_id, email, salt_hex, password_hash, role) VALUES (?1, ?2, ?3, ?4, ?5);",
        &[&user_id, &req.email, &salt, &hash, &role],
    ) {
        return bad_request(&format!("Could not create account: {e}"));
    }

    let _ = db.execute_params(
        "UPDATE invites SET used = 1 WHERE token = ?1;",
        &[&req.invite_token],
    );

    let session_token = format!("sess-{}", Uuid::new_v4());
    let user = UserAccount {
        user_id: user_id.clone(),
        email: req.email.clone(),
        role: role.clone(),
    };
    let _ = db.execute_params(
        "INSERT INTO sessions (session_id, user_id, email, role) VALUES (?1, ?2, ?3, ?4);",
        &[&session_token, &user.user_id, &user.email, &user.role],
    );

    let cookie = format!(
        "fsim_session={session_token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800"
    );
    let mut resp = Json(AuthResponse {
        session_token,
        user,
    })
    .into_response();
    resp.headers_mut()
        .insert(header::SET_COOKIE, cookie.parse().unwrap());
    resp
}

async fn api_create_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteRequest>,
) -> Response {
    let Some(user) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    if user.role != "admin" {
        return unauthorized("Admin role required to generate invite links");
    }
    let token = format!("inv-{}", Uuid::new_v4());
    let db = state.db.lock().unwrap();
    let _ = db.execute_params(
        "INSERT INTO invites (token, role, used) VALUES (?1, ?2, 0);",
        &[&token, &req.role],
    );
    Json(serde_json::json!({ "invite_token": token, "role": req.role })).into_response()
}

async fn api_create_reset_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateResetRequest>,
) -> Response {
    let Some(user) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    if user.role != "admin" {
        return unauthorized("Admin role required to generate password reset links");
    }
    let token = format!("rst-{}", Uuid::new_v4());
    let db = state.db.lock().unwrap();
    let _ = db.execute_params(
        "INSERT INTO password_resets (token, email, used) VALUES (?1, ?2, 0);",
        &[&token, &req.email],
    );
    Json(serde_json::json!({ "reset_token": token, "email": req.email })).into_response()
}

async fn api_use_reset_link(
    State(state): State<AppState>,
    Json(req): Json<UseResetRequest>,
) -> Response {
    let db = state.db.lock().unwrap();
    let Ok(rows) = db.query_rows(
        "SELECT email, used FROM password_resets WHERE token = ?1;",
        &[&req.reset_token],
    ) else {
        return bad_request("Invalid reset token");
    };
    let Some(row) = rows.first() else {
        return bad_request("Invalid reset token");
    };
    if row[1] != "0" {
        return bad_request("Reset token already used");
    }
    let email = row[0].clone();
    let salt = format!("salt-{}", Uuid::new_v4());
    let hash = reference::reference_hash_password(&req.new_password, &salt);
    let _ = db.execute_params(
        "UPDATE users SET salt_hex = ?1, password_hash = ?2, failed_attempts = 0 WHERE email = ?3;",
        &[&salt, &hash, &email],
    );
    let _ = db.execute_params(
        "UPDATE password_resets SET used = 1 WHERE token = ?1;",
        &[&req.reset_token],
    );
    Json(serde_json::json!({ "ok": true, "email": email })).into_response()
}

async fn api_catalogs(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let cells = full_starter_catalog();
    let preview_models = cells
        .iter()
        .filter_map(|c| catalog_preview_model(c).map(|m| (c.catalog_id.clone(), m)))
        .collect();
    Json(StaticCatalogBundle {
        buildings: starter_building_listings(),
        cells,
        markets: starter_markets(),
        certifications: starter_certifications(),
        preview_models,
    })
    .into_response()
}

async fn api_list_games(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(user) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let games = state.games.lock().unwrap();
    let mut list = Vec::new();
    for g in games.values() {
        if g.owner_id == user.user_id || user.role == "admin" {
            let rw = g.world.runway_projection();
            list.push(GameSummaryItem {
                game_id: g.world.game_id.clone(),
                owner_id: g.owner_id.clone(),
                scenario_name: g.world.scenario_name.clone(),
                game_seed: g.world.game_seed,
                sim_time_ms: g.world.now.0,
                cash_tenth_cents: g.world.ledger.cash_balance().0,
                runway_days_p50: rw.runway_days_p50,
                plant_count: g.world.plants.len(),
                bankrupt: rw.bankrupt,
            });
        }
    }
    Json(list).into_response()
}

async fn api_create_game(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateGameRequest>,
) -> Response {
    let Some(user) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let game_id = format!("game-{}", Uuid::new_v4().simple());
    let mut world = SimWorld::new(&game_id, &req.scenario_name, req.game_seed);
    world.auto_pause_on_alerts = req.auto_pause_on_alerts;

    let init_ck = CheckpointMeta {
        checkpoint_id: "ck-init".to_string(),
        name: "Initial Seed Checkpoint (Day 0)".to_string(),
        is_autosave: true,
        sim_time_ms: 0,
        log_seq: 0,
        cash_tenth_cents: world.ledger.cash_balance().0,
        runway_days_p50: world.runway_projection().runway_days_p50,
    };
    let mut ck_states = BTreeMap::new();
    ck_states.insert("ck-init".to_string(), world.clone());

    let inst = GameInstance {
        owner_id: user.user_id.clone(),
        world,
        checkpoints: vec![init_ck],
        checkpoint_states: ck_states,
        last_skip_summary: None,
        speed_multiplier: 0,
    };
    let _ = state.persist_game(&inst);
    let snap = inst.to_client_snapshot();
    state.games.lock().unwrap().insert(game_id, inst);
    Json(snap).into_response()
}

async fn api_get_game(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let games = state.games.lock().unwrap();
    let Some(inst) = games.get(&game_id) else {
        return bad_request("Game not found");
    };
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_apply_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(cmd): Json<PlantCommand>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let mut games = state.games.lock().unwrap();
    let Some(inst) = games.get_mut(&game_id) else {
        return bad_request("Game not found");
    };
    if let Err(reason) = inst.world.apply_command(cmd) {
        return bad_request(&reason);
    }
    let _ = state.persist_game(inst);
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_skip_ahead(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<SkipAheadRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let mut games = state.games.lock().unwrap();
    let Some(inst) = games.get_mut(&game_id) else {
        return bad_request("Game not found");
    };
    let summary = inst.world.skip_ahead(req.max_horizon_ms);
    inst.last_skip_summary = Some(summary);

    // Autosave checkpoint after every skip-ahead
    let ck_id = format!("ck-auto-{}", inst.checkpoints.len() + 1);
    let rw = inst.world.runway_projection();
    inst.checkpoints.push(CheckpointMeta {
        checkpoint_id: ck_id.clone(),
        name: format!(
            "Autosave Day {:.1}",
            (inst.world.now.0 as f64) / (SimTime::DAY_MS as f64)
        ),
        is_autosave: true,
        sim_time_ms: inst.world.now.0,
        log_seq: inst.world.gateway.log_entries.len() as u64,
        cash_tenth_cents: inst.world.ledger.cash_balance().0,
        runway_days_p50: rw.runway_days_p50,
    });
    inst.checkpoint_states
        .insert(ck_id, inst.world.clone());

    let _ = state.persist_game(inst);
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_step_small(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<SkipAheadRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let mut games = state.games.lock().unwrap();
    let Some(inst) = games.get_mut(&game_id) else {
        return bad_request("Game not found");
    };
    let target = inst.world.now.add_ms(req.max_horizon_ms.max(1_000));
    inst.world.advance_to(target);
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_validate_placement(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<ValidatePlacementRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let games = state.games.lock().unwrap();
    let Some(inst) = games.get(&game_id) else {
        return bad_request("Game not found");
    };
    match inst.world.validate_placement(
        &req.plant_id,
        &req.catalog_id,
        req.x_mm,
        req.y_mm,
        req.rotation_deg,
        req.ignore_cell_id.as_deref(),
    ) {
        Ok(()) => Json(ValidatePlacementResponse {
            valid: true,
            errors: Vec::new(),
        })
        .into_response(),
        Err(errors) => Json(ValidatePlacementResponse {
            valid: false,
            errors,
        })
        .into_response(),
    }
}

async fn api_save_checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<SaveCheckpointRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let mut games = state.games.lock().unwrap();
    let Some(inst) = games.get_mut(&game_id) else {
        return bad_request("Game not found");
    };
    let ck_id = format!("ck-{}", inst.checkpoints.len() + 1);
    let rw = inst.world.runway_projection();
    inst.checkpoints.push(CheckpointMeta {
        checkpoint_id: ck_id.clone(),
        name: req.name,
        is_autosave: req.is_autosave,
        sim_time_ms: inst.world.now.0,
        log_seq: inst.world.gateway.log_entries.len() as u64,
        cash_tenth_cents: inst.world.ledger.cash_balance().0,
        runway_days_p50: rw.runway_days_p50,
    });
    inst.checkpoint_states
        .insert(ck_id, inst.world.clone());
    let _ = state.persist_game(inst);
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_rewind_checkpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<RewindCheckpointRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let mut games = state.games.lock().unwrap();
    let Some(inst) = games.get_mut(&game_id) else {
        return bad_request("Game not found");
    };
    let Some(saved) = inst.checkpoint_states.get(&req.checkpoint_id).cloned() else {
        return bad_request("Checkpoint not found");
    };
    inst.world = saved;
    inst.last_skip_summary = None;
    let _ = state.persist_game(inst);
    Json(inst.to_client_snapshot()).into_response()
}

async fn api_report_problem(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(game_id): AxumPath<String>,
    Json(req): Json<ProblemReportRequest>,
) -> Response {
    let Some(_) = state.authenticate_headers(&headers) else {
        return unauthorized("Sign-in required");
    };
    let games = state.games.lock().unwrap();
    let Some(inst) = games.get(&game_id) else {
        return bad_request("Game not found");
    };
    let bundle = ProblemReportBundle {
        report_id: format!("rep-{}", Uuid::new_v4().simple()),
        game_id: inst.world.game_id.clone(),
        game_seed: inst.world.game_seed,
        sim_time_ms: inst.world.now.0,
        log_seq: inst.world.gateway.log_entries.len() as u64,
        latest_checkpoint_id: inst.checkpoints.last().map(|c| c.checkpoint_id.clone()),
        note: req.note,
    };
    let report_path = state
        .data_dir
        .join("games")
        .join(&game_id)
        .join(format!("{}.report.json", bundle.report_id));
    if let Ok(bytes) = serde_json::to_vec_pretty(&bundle) {
        let _ = reference::atomic_write_file(&report_path, &bytes);
    }
    Json(bundle).into_response()
}

// --- WebSocket Protocol Handler ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    Command { command: PlantCommand },
    SkipAhead { max_horizon_ms: i64 },
    Step { delta_ms: i64 },
    RequestSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    Snapshot {
        to_version: u64,
        snapshot: ClientGameSnapshot,
    },
    Error {
        reason: String,
    },
}

async fn ws_game_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AxumPath(game_id): AxumPath<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_session(socket, state, game_id))
}

async fn handle_ws_session(mut socket: WebSocket, state: AppState, game_id: String) {
    // Send initial full snapshot on connect
    let initial_msg = {
        let games = state.games.lock().unwrap();
        games.get(&game_id).map(|inst| WsServerMessage::Snapshot {
            to_version: inst.world.version,
            snapshot: inst.to_client_snapshot(),
        })
    };
    if let Some(msg) = initial_msg {
        if let Ok(txt) = serde_json::to_string(&msg) {
            let _ = socket.send(Message::Text(txt)).await;
        }
    } else {
        return;
    }

    while let Some(Ok(frame)) = socket.recv().await {
        let Message::Text(text) = frame else {
            continue;
        };
        let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) else {
            continue;
        };

        let response_msg = {
            let mut games = state.games.lock().unwrap();
            let Some(inst) = games.get_mut(&game_id) else {
                break;
            };
            match client_msg {
                WsClientMessage::Command { command } => match inst.world.apply_command(command) {
                    Ok(_) => {
                        let _ = state.persist_game(inst);
                        WsServerMessage::Snapshot {
                            to_version: inst.world.version,
                            snapshot: inst.to_client_snapshot(),
                        }
                    }
                    Err(reason) => WsServerMessage::Error { reason },
                },
                WsClientMessage::SkipAhead { max_horizon_ms } => {
                    let summary = inst.world.skip_ahead(max_horizon_ms);
                    inst.last_skip_summary = Some(summary);
                    let _ = state.persist_game(inst);
                    WsServerMessage::Snapshot {
                        to_version: inst.world.version,
                        snapshot: inst.to_client_snapshot(),
                    }
                }
                WsClientMessage::Step { delta_ms } => {
                    let target = inst.world.now.add_ms(delta_ms.max(1_000));
                    inst.world.advance_to(target);
                    WsServerMessage::Snapshot {
                        to_version: inst.world.version,
                        snapshot: inst.to_client_snapshot(),
                    }
                }
                WsClientMessage::RequestSnapshot => WsServerMessage::Snapshot {
                    to_version: inst.world.version,
                    snapshot: inst.to_client_snapshot(),
                },
            }
        };

        if let Ok(txt) = serde_json::to_string(&response_msg) {
            if socket.send(Message::Text(txt)).await.is_err() {
                break;
            }
        }
    }
}
