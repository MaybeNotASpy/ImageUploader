#![warn(unused_extern_crates)]
use std::{env, io::Error as IoError, thread};

use tokio::net::TcpListener;

use tokio::sync::mpsc;

mod connection_type;
mod credentials;
mod upload_server;
mod db;

use crate::connection_type::ConnectionType;
use crate::upload_server::accept_upload_connection;
use crate::db::{DbMessage, DbPipeIn};

async fn select_server(addr: String, access_port: String, connection_type: ConnectionType, db_tx: DbPipeIn) {
    loop {
        println!("Binding {connection_type} server to address {addr}, port {access_port}...");
        let listener = TcpListener::bind(format!("{addr}:{access_port}")).await;
        if listener.is_err() {
            println!(
                "Error while binding {connection_type} server: {}",
                listener.err().unwrap()
            );
            continue;
        }
        let listener = listener.unwrap();
        loop {
            let acception = listener.accept().await;
            match acception {
                Ok(_) => {}
                Err(e) if (e.kind() == tokio::io::ErrorKind::NotConnected) => {
                    println!("Error in TCPListener connection: {}", e);
                    break;
                }
                Err(_) => {
                    println!(
                        "Error while accepting connection: {}",
                        acception.err().unwrap()
                    );
                    continue;
                }
            }
            println!("Access server listening on address {addr}, port {access_port}");
            let (stream, addr) = acception.unwrap();
            let connection_clone = connection_type.clone();
            let db_tx_clone = db_tx.clone();
            tokio::spawn(async move {
                match connection_clone {
                    ConnectionType::Upload => {
                        accept_upload_connection(stream, addr, db_tx_clone).await;
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    // Gets the address and port from the command line arguments
    let address: String = env::args().nth(1).unwrap_or_else(|| "0.0.0.0".to_string());
    let upload_port: String = env::args().nth(2).unwrap_or_else(|| "8080".to_string());

    let (
        db_tx, 
        db_rx,
    ) = mpsc::unbounded_channel::<DbMessage>();

    let _db_handle = thread::spawn(move || { 
        db::run(db_rx);
    });

    let upload_handle = tokio::spawn(select_server(
        address.clone(),
        upload_port.clone(),
        ConnectionType::Upload,
        db_tx.clone(),
    ));

    upload_handle.await?;
    Ok(())
}
