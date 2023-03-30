#![warn(unused_extern_crates)]
use std::{
    env,
    error::Error,
    io::Error as IoError,
    net::SocketAddr,
    path::Path,
    ops::Deref
};

use webp::Encoder;

use uuid::Uuid;

use futures_util::StreamExt;
use serde::Deserialize;

use tokio::net::{TcpListener, TcpStream};
use tokio::fs::{create_dir_all, write};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Deserialize)]
struct Credentials {
    organization: String,
    username: String,
    mission: String,
}

impl Credentials {
    // Decodes the credentials from the message's json data
    fn read_from_message(msg: Message) -> Result<Credentials, Box<dyn Error>> {
        let credentials: Credentials = serde_json::from_str(
            &msg.into_text()?
        )?;
        Ok(credentials)
    }
}

async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr) {
    if let Err(e) = handle_connection(raw_stream, addr).await {
        println!("Error while handling connection: {e}");
    }
}

// Reads the first message from the client, which should contain the credentials.
async fn read_first_message(ws_stream: &mut WebSocketStream<TcpStream>) 
    -> Result<String, Box<dyn Error>> 
{
    let msg = ws_stream.next().await.ok_or("Did not receive credentials.")?;
    if msg.is_err() {
        let err_txt = format!("Error while reading credentials: {}", msg.err().unwrap());
        return Err(err_txt.into());
    }
    let msg = msg.unwrap();
    // Reads the credentials from the message. Expects it to be in json format.
    let credentials = Credentials::read_from_message(msg)?;
    // Defines the directory to store the image(s)
    let path = format!(
        "messages/{}/{}/{}/",
        credentials.organization, credentials.username, credentials.mission
    );
    // Creates the directory if it does not exist
    create_dir_all(Path::new(&path)).await?;
    Ok(path)
}

async fn process_image(msg: Message, path: &str) -> Result<(), Box<dyn Error>> {
    let data = Encoder::from_image( // Converts the image to webp format
            &image::load_from_memory( // Loads the image from the message's data
                &msg.into_data() // Converts the message to a vector of bytes
            )?
        )?
        .encode_lossless() // Encodes the image losslessly
        .deref() // Converts the image to a slice of bytes
        .to_vec();  // Converts the slice to a vector of bytes. Must copy the data
                    // to a new vector because WebPMemory cannot move between threads.
    let filename = format!("{}.webp", Uuid::new_v4());
    let path = format!("{path}{filename}");
    write(path, data).await?;
    Ok(())
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    println!("New connection from {addr}");

    // Upgrades the TCP connection to a websocket connection
    let mut ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
    println!("New WebSocket connection: {addr}");

    // Reads the first message from the client, which should contain the credentials.
    let path = read_first_message(&mut ws_stream).await?;

    // Reads the rest of the messages from the client and saves them as webp images
    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_close() {
            break;
        }
        process_image(msg, &path).await?;
    }

    println!("{addr} disconnected");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    // Gets the address and port from the command line arguments
    let address: String = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port: String = env::args().nth(2).unwrap_or_else(|| "8080".to_string());

    println!("Binding server to address {address}, port {port}...");
    // Binds the server to the address and port
    let listener = TcpListener::bind(format!("{address}:{port}")).await?;
    println!("Listening on address {address}, port {port}");

    // Accepts incoming connections and spawns a new task for each connection
    loop {
        let (stream, addr) = listener.accept().await?;
        tokio::spawn(async move {
            accept_connection(stream, addr).await;
        });
    }
}