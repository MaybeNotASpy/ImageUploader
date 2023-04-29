// Module: download_server
// Path: ImageUploader/src/download_server.rs

// This file contains the code for the server that retrieves the images from the server.

use std::{error::Error, sync::Arc};
use tokio::fs::read;
use axum::extract::ws::{WebSocket, Message};

use crate::credentials::Credentials;
use crate::db::{DbMessage, DbPipeIn, QueryType};

pub async fn handle_download_connection(
    mut socket: WebSocket,
    db_tx: Arc<DbPipeIn>,
    credentials: Credentials,
) -> Result<(), Box<dyn Error>> {
    log::info!("New Download connection from {}", credentials.username);

    let (tx, rx) = tokio::sync::oneshot::channel::<Vec<String>>();

    db_tx.send(DbMessage{
        credentials: credentials.clone(),
        query_type: QueryType::Select,
        return_tx: Some(tx),
    }).unwrap_or_else(|e| log::error!("Error while sending message to database thread: {}", e));

    let paths = rx.await.unwrap_or(Vec::new());

    for path in paths {
        let data = read(path).await?;
        socket.send(Message::Binary(data)).await?;
    }
    socket.close().await?;

    log::info!("{} disconnected", credentials.username);
    Ok(())
}
