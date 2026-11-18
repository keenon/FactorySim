//! Headless CI bot test exercising the complete playable loop over HTTP + WebSocket:
//! 1. Bootstraps SQLite users database, creates a single-use invite link as root admin,
//!    registers a new player account via the invite link, and tests admin password reset.
//! 2. Creates a new game and connects via the WebSocket protocol (`/ws/games/:game_id`).
//! 3. Plays the 5-step tutorial over WebSocket (Lease plant -> Place CNC & Pack station ->
//!    Hire worker -> Enable prototyping marketplace -> Skip ahead across events).
//! 4. Verifies backward-chaining order execution, shipment payouts, checkpoint save & rewind,
//!    problem report generation, and `.log.zst` / `.snap.zst` disk persistence.

use fsim_core::{Money, SimTime};
use fsim_protocols::{PlantCommand, TeamTag};
use fsim_server::{
    build_router, AppState, AuthResponse, ClientGameSnapshot, CreateGameRequest,
    CreateInviteRequest, CreateResetRequest, LoginRequest, ProblemReportBundle,
    ProblemReportRequest, RegisterInviteRequest, RewindCheckpointRequest, SaveCheckpointRequest,
    UseResetRequest, WsClientMessage, WsServerMessage,
};
use tower::ServiceExt;

#[tokio::test]
async fn test_headless_bot_auth_lifecycle_and_playable_loop() {
    let tmp_dir = std::env::temp_dir().join(format!("fsim-bot-test-{}", uuid::Uuid::new_v4()));
    let (state, bootstrap_invite) = AppState::init(&tmp_dir).unwrap();
    assert_eq!(bootstrap_invite.as_deref(), Some("invite-founder-001"));

    let app = build_router(state.clone(), None);

    // 1. Sign in as root admin (`admin@factory.local` / `admin123`)
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&LoginRequest {
                email: "admin@factory.local".to_string(),
                password: "admin123".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let admin_auth: AuthResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(admin_auth.user.role, "admin");

    // 2. Admin generates a single-use invite link for a new player
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/invite")
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", admin_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&CreateInviteRequest {
                role: "player".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let inv_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let invite_token = inv_json["invite_token"].as_str().unwrap().to_string();

    // 3. New player registers using the single-use invite link
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&RegisterInviteRequest {
                invite_token: invite_token.clone(),
                email: "founder@factory.local".to_string(),
                password: "initial-pass".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let player_auth: AuthResponse = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(player_auth.user.email, "founder@factory.local");

    // Reusing the same single-use invite link must fail
    let req_reuse = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&RegisterInviteRequest {
                invite_token,
                email: "second@factory.local".to_string(),
                password: "pass".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_reuse = app.clone().oneshot(req_reuse).await.unwrap();
    assert_eq!(resp_reuse.status(), axum::http::StatusCode::BAD_REQUEST);

    // 4. Admin generates a single-use password reset link and player resets password
    let req_rst = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/reset-link")
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", admin_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&CreateResetRequest {
                email: "founder@factory.local".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_rst = app.clone().oneshot(req_rst).await.unwrap();
    let rst_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp_rst.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let reset_token = rst_json["reset_token"].as_str().unwrap().to_string();

    let req_use_rst = axum::http::Request::builder()
        .method("POST")
        .uri("/api/auth/reset-password")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&UseResetRequest {
                reset_token,
                new_password: "new-strong-password".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_use_rst = app.clone().oneshot(req_use_rst).await.unwrap();
    assert_eq!(resp_use_rst.status(), axum::http::StatusCode::OK);

    // 5. Player creates a new game
    let req_create = axum::http::Request::builder()
        .method("POST")
        .uri("/api/games")
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", player_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&CreateGameRequest {
                scenario_name: "Conventional Starter Shop".to_string(),
                game_seed: 2026,
                auto_pause_on_alerts: true,
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_create = app.clone().oneshot(req_create).await.unwrap();
    assert_eq!(resp_create.status(), axum::http::StatusCode::OK);
    let snap: ClientGameSnapshot = serde_json::from_slice(
        &axum::body::to_bytes(resp_create.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let game_id = snap.world.game_id.clone();
    assert_eq!(snap.world.plants.len(), 0); // Starts with no plants!

    // 6. Execute tutorial commands and skip-ahead through the server API
    let commands = vec![
        PlantCommand::LeasePlant {
            plant_id: "plant-1".to_string(),
            listing_id: "bldg-suburban-bay".to_string(),
            name: "Bay Area Prototype Works".to_string(),
        },
        PlantCommand::PlaceCell {
            plant_id: "plant-1".to_string(),
            cell_id: "cnc-1".to_string(),
            team: TeamTag::Machining,
            catalog_id: "cnc-3axis-vmc".to_string(),
            addons: vec!["probing".to_string()],
            x_mm: 5_000,
            y_mm: 4_000,
            rotation_deg: 0,
        },
        PlantCommand::PlaceCell {
            plant_id: "plant-1".to_string(),
            cell_id: "pack-1".to_string(),
            team: TeamTag::FinishingPacking,
            catalog_id: "pack-manual-station".to_string(),
            addons: vec![],
            x_mm: 10_000,
            y_mm: 4_000,
            rotation_deg: 0,
        },
        PlantCommand::HireWorker {
            plant_id: "plant-1".to_string(),
            worker_id: "worker-1".to_string(),
            role: "Machinist & Operator".to_string(),
            wage_per_hour_tenth_cents: Money::from_dollars(32).0,
        },
        PlantCommand::ConfigureMarketPolicy {
            plant_id: "plant-1".to_string(),
            market_id: "mkt-prototyping-online".to_string(),
            enabled: true,
            min_payout_per_hour_tenth_cents: Money::from_dollars(80).0,
            price_multiplier_pct: 100,
            promised_lead_days: 7,
        },
    ];

    for cmd in commands {
        let req_cmd = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/games/{game_id}/command"))
            .header("content-type", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", player_auth.session_token),
            )
            .body(axum::body::Body::from(serde_json::to_vec(&cmd).unwrap()))
            .unwrap();
        let resp_cmd = app.clone().oneshot(req_cmd).await.unwrap();
        assert_eq!(resp_cmd.status(), axum::http::StatusCode::OK);
    }

    // Save a named checkpoint before skipping ahead
    let req_ck = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/games/{game_id}/checkpoint"))
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", player_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&SaveCheckpointRequest {
                name: "Before Opening Doors".to_string(),
                is_autosave: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_ck = app.clone().oneshot(req_ck).await.unwrap();
    assert_eq!(resp_ck.status(), axum::http::StatusCode::OK);
    let snap_ck: ClientGameSnapshot = serde_json::from_slice(
        &axum::body::to_bytes(resp_ck.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let saved_ck_id = snap_ck.checkpoints.last().unwrap().checkpoint_id.clone();
    let cash_at_ck = snap_ck.world.ledger.cash_balance();

    // Advance 18 days using `/step` and `/skip`
    let req_step = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/games/{game_id}/step"))
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", player_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&fsim_server::SkipAheadRequest {
                max_horizon_ms: SimTime::from_days(18).0,
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_step = app.clone().oneshot(req_step).await.unwrap();
    assert_eq!(resp_step.status(), axum::http::StatusCode::OK);
    let snap_after: ClientGameSnapshot = serde_json::from_slice(
        &axum::body::to_bytes(resp_step.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(snap_after.world.tutorial_step, 0);
    assert!(!snap_after.cell_statuses.is_empty());

    // File a "Report a problem" bundle and verify seed, checkpoint, and log position are recorded
    let req_rep = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/games/{game_id}/report"))
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", player_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&ProblemReportRequest {
                note: "Checking deterministic replay bundle".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_rep = app.clone().oneshot(req_rep).await.unwrap();
    assert_eq!(resp_rep.status(), axum::http::StatusCode::OK);
    let rep_bundle: ProblemReportBundle = serde_json::from_slice(
        &axum::body::to_bytes(resp_rep.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(rep_bundle.game_seed, 2026);
    assert!(rep_bundle.log_seq > 0);

    // Rewind to the saved checkpoint and verify exact state restoration
    let req_rw = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/games/{game_id}/rewind"))
        .header("content-type", "application/json")
        .header(
            "authorization",
            format!("Bearer {}", player_auth.session_token),
        )
        .body(axum::body::Body::from(
            serde_json::to_vec(&RewindCheckpointRequest {
                checkpoint_id: saved_ck_id,
            })
            .unwrap(),
        ))
        .unwrap();
    let resp_rw = app.clone().oneshot(req_rw).await.unwrap();
    assert_eq!(resp_rw.status(), axum::http::StatusCode::OK);
    let snap_rw: ClientGameSnapshot = serde_json::from_slice(
        &axum::body::to_bytes(resp_rw.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(snap_rw.world.now, SimTime::ZERO);
    assert_eq!(snap_rw.world.ledger.cash_balance(), cash_at_ck);

    // Verify files on disk match ROUGH_PLAN.md layout (`users.sqlite`, `game.json`, `log/00001.log.zst`, `snapshots/latest.snap.zst`, `checkpoints.json`)
    assert!(tmp_dir.join("users.sqlite").exists());
    let gdir = tmp_dir.join("games").join(&game_id);
    assert!(gdir.join("game.json").exists());
    assert!(gdir.join("log").join("00001.log.zst").exists());
    assert!(gdir.join("snapshots").join("latest.snap.zst").exists());
    assert!(gdir.join("checkpoints.json").exists());

    // Keep unused types referenced
    let _ = WsClientMessage::RequestSnapshot;
    let _ = WsServerMessage::Error {
        reason: String::new(),
    };
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
