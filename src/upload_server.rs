// Module: upload_server
// Path: ImageUploader/src/upload_server.rs

// This file contains the code for the server that receives the images from the clients.

use futures_util::StreamExt;
use std::{error::Error, net::SocketAddr, ops::Deref, path::Path};
use tokio::fs::{create_dir_all, write};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use uuid::Uuid;
use webp::Encoder;

use crate::credentials::Credentials;
use crate::db::{DbMessage, DbPipeIn, QueryType};

// Reads the first message from the client, which should contain the credentials.
async fn read_first_message(
    ws_stream: &mut WebSocketStream<TcpStream>,
) -> Result<Credentials, Box<dyn Error>> {
    let msg = ws_stream
        .next()
        .await
        .ok_or("Did not receive credentials.")?;
    if msg.is_err() {
        let err_txt = format!("Error while reading credentials: {}", msg.err().unwrap());
        return Err(err_txt.into());
    }
    let msg = msg.unwrap();
    // Reads the credentials from the message. Expects it to be in json format.
    let credentials = Credentials::read_from_message(msg)?;
    Ok(credentials)
}

pub async fn accept_upload_connection(raw_stream: TcpStream, addr: SocketAddr, db_tx: DbPipeIn) {
    if let Err(e) = handle_upload_connection(raw_stream, addr, db_tx).await {
        println!("Error while handling Upload connection: {e}");
    } else {
        println!("Upload connection handled successfully");
    }
}

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

async fn handle_upload_connection(
    raw_stream: TcpStream,
    addr: SocketAddr,
    db_tx: DbPipeIn,
) -> Result<(), Box<dyn Error>> {
    println!("New Upload connection from {addr}");

    // Upgrades the TCP connection to a websocket connection
    let mut ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
    println!("New Upload WebSocket connection: {addr}");

    // Reads the first message from the client, which should contain the credentials.
    let credentials = read_first_message(&mut ws_stream).await?;

    // Defines the directory to store the image(s)
    let path = format!(
        "images/{}/{}/{}/",
        credentials.organization, credentials.username, credentials.mission
    );
    // Creates the directory if it does not exist
    create_dir_all(Path::new(&path)).await?;

    // Reads the rest of the messages from the client and saves them as webp images
    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_close() {
            break;
        }
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
            Err(e) => println!("Error while sending image credentials to db: {e}"),
        }
    }

    println!("{addr} disconnected");
    Ok(())
}
