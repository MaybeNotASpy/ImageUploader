#![warn(unused_extern_crates)]
use std::fs::read;
use std::net::SocketAddr;
use std::{error::Error as Error, thread, sync::Arc};

use axum::extract::ConnectInfo;
use futures::{stream};
use tokio::sync::mpsc;
use tokio::sync::oneshot;


use flexi_logger::{Duplicate, Logger, FileSpec, Criterion, Naming, Cleanup, WriteMode, Age};

use axum::{
    extract::ws::WebSocketUpgrade,
    extract::State,
    extract::Query,
    routing::get,
    response::Response,
    Router,
    Server,
    http::StatusCode,
    response::IntoResponse,
    body::StreamBody
};

mod credentials;
mod upload_server;
mod download_server;
mod db;

use crate::upload_server::handle_upload_connection;
use crate::db::{DbMessage, DbPipeIn, QueryType};
use crate::credentials::Credentials;
use crate::download_server::handle_download_connection;

fn start_logger() -> Result<(), Box<dyn Error>> {
    Logger::try_with_str("info")?
    .log_to_file(FileSpec::default()
         .directory("logs")
         .basename("image_uploader")
     )
     .rotate(                      // If the program runs long enough,
         Criterion::Age(Age::Day), // - create a new file every day
         Naming::Timestamps,       // - let the rotated files have a timestamp in their name
         Cleanup::KeepLogFiles(7), // - keep at most 7 log files
     )
     .duplicate_to_stderr(Duplicate::Warn)
     .write_mode(WriteMode::Async)
     .start()?;
    Ok(())
}

async fn ws_upload_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<DbPipeIn>>,
    Query(creds): Query<Credentials>,
    ws: WebSocketUpgrade,
) -> Response {
    log::info!("Incoming WebSocket request from {}...", addr);
    ws.on_upgrade(|socket| async move {
        match handle_upload_connection(socket, state, creds).await {
            Ok(_) => log::info!("Upload connection closed"),
            Err(e) => log::error!("Error while handling upload connection: {}", e),
        };
    })
}

async fn download_handler(
    Query(creds): Query<Credentials>,
    State(state): State<Arc<DbPipeIn>>,
) -> Response {
    log::info!("Incoming download request for {}/{}/{}...", creds.organization, creds.username, creds.mission);

    let (tx, rx) = oneshot::channel();

    match state.send(DbMessage {
        credentials: creds,
        query_type: QueryType::Select,
        return_tx: Some(tx),  
    }) {
        Ok(_) => (),
        Err(e) => log::error!("Error while sending download request to database: {}", e),
    };

    let image_filepaths = match rx.await {
        Ok(image_filepaths) => image_filepaths,
        Err(e) => {
            log::error!("Error while receiving images from database: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error while receiving images from database").into_response();
        }
    };

    let image_map = image_filepaths.into_iter().map(|image_filepath| {
        read(image_filepath)
    });

    let image_stream = stream::iter(image_map);
    
    StreamBody::new(image_stream).into_response()
}

async fn ws_download_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<DbPipeIn>>,
    Query(creds): Query<Credentials>,
    ws: WebSocketUpgrade,
) -> Response {
    log::info!("Incoming WebSocket request from {}...", addr);
    ws.on_upgrade(|socket| async move {
        match handle_download_connection(socket, state, creds).await {
            Ok(_) => log::info!("Download connection closed"),
            Err(e) => log::error!("Error while handling download connection: {}", e),
        };
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    start_logger()?;

    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid port number");


    let (
        db_tx, 
        db_rx,
    ) = mpsc::unbounded_channel::<DbMessage>();

    let _db_handle = thread::spawn(move || { 
        db::run(db_rx);
    });

    let db_arc = Arc::new(db_tx);

    let app: Router<()> = Router::new()
        .route("/upload/ws", get(ws_upload_handler))
        .route("/download", get(download_handler))
        .route("/download/ws", get(ws_download_handler))
        .with_state(db_arc);

    Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}
