// Module: upload_server
// Path: ImageUploader/src/upload_server.rs

// This file contains the code for the server that receives the images from the clients.

use std::{error::Error, ops::Deref, path::Path, sync::Arc};
use tokio::fs::{create_dir_all, write};
use uuid::Uuid;
use webp::Encoder;
use axum::extract::ws::{WebSocket, Message};

use crate::credentials::Credentials;
use crate::db::{DbMessage, DbPipeIn, QueryType};

async fn process_image(msg: Message, path: &str) -> Result<Uuid, Box<dyn Error>> {
    let data = Encoder::from_image(
        // Converts the image to webp format
        &image::load_from_memory(
            // Loads the image from the message's data
            &msg.into_data(), // Converts the message to a vector of bytes
        )?,
    )?
    .encode_lossless() // Encodes the image losslessly
    .deref() // Converts the image to a slice of bytes
    .to_vec(); // Converts the slice to a vector of bytes. Must copy the data
               // to a new vector because WebPMemory cannot move between threads.
    let id = Uuid::new_v4();
    let path = format!("{path}{id}.webp");
    write(path, data).await?;
    Ok(id)
}

pub async fn handle_upload_connection(
    mut socket: WebSocket,
    db_tx: Arc<DbPipeIn>,
    credentials: Credentials,
) -> Result<(), Box<dyn Error>> {
    log::info!("New Upload connection from {}", credentials.username);

    // Defines the directory to store the image(s)
    let path = format!(
        "images/{}/{}/{}/",
        credentials.organization, credentials.username, credentials.mission
    );
    // Creates the directory if it does not exist
    create_dir_all(Path::new(&path)).await?;

    // Reads the rest of the messages from the client and saves them as webp images
    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(Message::Close(_)) => break,
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Error while receiving message from client: {e}");
                continue;
            }
        };
        let id = process_image(msg, &path).await?;
        let image_credentials = Credentials {
            organization: credentials.organization.clone(),
            username: credentials.username.clone(),
            mission: credentials.mission.clone(),
            id: Some(id.to_string()),
            filepath: Some(format!("{path}{id}.webp")),
        };
        match db_tx.send(DbMessage {
            credentials: image_credentials,
            query_type: QueryType::Insert,
            return_tx: None,
        }) {
            Ok(_) => (),
            Err(e) => log::error!("Error while sending image credentials to db: {e}"),
        }
    }

    log::info!("{} disconnected", credentials.username);
    Ok(())
}
