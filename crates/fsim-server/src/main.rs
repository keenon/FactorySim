use fsim_server::{build_router, AppState};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data_dir = std::env::var("FSIM_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let web_dist = std::env::var("FSIM_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("web/dist"));

    let (state, bootstrap_invite) = AppState::init(&data_dir).map_err(|e| anyhow::anyhow!(e))?;
    if let Some(inv) = bootstrap_invite {
        println!("==============================================================");
        println!("Factory Sim initialized first-run root admin in data/users.sqlite");
        println!("  Admin login : admin@factory.local / admin123");
        println!("  Starter invite token: {inv}");
        println!("==============================================================");
    }

    let addr = std::env::var("FSIM_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:4170".to_string());
    let app = build_router(state, Some(web_dist));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Factory Sim server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
