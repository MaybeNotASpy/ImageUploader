use rusqlite::{Connection as SqliteConnection};
use tokio::sync::{oneshot, mpsc};
use crate::credentials::Credentials;

pub struct DbMessage {
    pub credentials: Credentials, 
    pub query_type: QueryType,
    pub return_tx: Option<oneshot::Sender<Vec<String>>>
}
pub type DbPipeOut = mpsc::UnboundedReceiver<DbMessage>;
pub type DbPipeIn = mpsc::UnboundedSender<DbMessage>;

pub enum QueryType {
    Insert,
    Select,
    Update,
    Delete,
}


pub fn run(mut rx: DbPipeOut) {
    let db_connection = SqliteConnection::open("images.db").unwrap();
    db_connection.execute(
        "CREATE TABLE IF NOT EXISTS images (
            id INTEGER PRIMARY KEY,
            organization TEXT NOT NULL,
            username TEXT NOT NULL,
            mission TEXT NOT NULL,
            id TEXT NOT NULL,
            filepath TEXT NOT NULL
        )",
        [],
    ).unwrap();

    loop {
        let received: Option<DbMessage> = rx.blocking_recv();
        if received.is_none() {
            continue;
        }
        let message = received.unwrap();
        let creds = message.credentials;
        let query_type = message.query_type;
        let return_tx = message.return_tx;
        match query_type {
            QueryType::Insert => {
                if creds.filepath.is_none() {
                    panic!("Error: Insert called when filepath is none!");
                }
                if creds.id.is_none() {
                    panic!("Error: Insert called when id is none!");
                }
                let query = format!(
                    "INSERT INTO images (organization, username, mission, id, filepath) VALUES ('{organization}', '{username}', '{mission}', '{id}', '{filepath}')",
                    organization = creds.organization,
                    username = creds.username,
                    mission = creds.mission,
                    id = creds.id.unwrap(),
                    filepath = creds.filepath.unwrap()
                );
                match db_connection.execute(&query, []) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error while executing query {}: {}", query, e);
                        continue;
                    }
                }
            }
            QueryType::Select => {
                if return_tx.is_none() {
                    panic!("Error: Select called when return_tx is none!");
                }
                let query = format!(
                    "SELECT * FROM images WHERE organization='{organization}' AND username='{username}' AND mission='{mission}'",
                    organization = creds.organization,
                    username = creds.username,
                    mission = creds.mission,
                );
                let mut stmt = db_connection.prepare(&query).unwrap();
                let mut rows = stmt.query([]).unwrap();
                let mut results: Vec<String> = Vec::new();
                while let Some(row) = rows.next().unwrap() {
                    let filepath: String = row.get(4).unwrap();
                    results.push(filepath);
                }
                return_tx.unwrap().send(results);
            }
            QueryType::Update => {
                if creds.id.is_none() {
                    panic!("Error: Update called when id is none!");
                }
                let query = format!(
                    "UPDATE images SET organization='{organization}', username='{username}', mission='{mission}' WHERE id='{id}''",
                    organization = creds.organization,
                    username = creds.username,
                    mission = creds.mission,
                    id = creds.id.unwrap()
                );
                match db_connection.execute(&query, []) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error while executing query {}: {}", query, e);
                        continue;
                    }
                }
            }
            QueryType::Delete => {
                if creds.id.is_none() {
                    panic!("Error: Delete called when id is none!");
                }
                let query = format!(
                    "DELETE FROM images WHERE organization='{organization}' AND username='{username}' AND mission='{mission}' AND id='{id}'",
                    organization = creds.organization,
                    username = creds.username,
                    mission = creds.mission,
                    id = creds.id.unwrap()
                );
                match db_connection.execute(&query, []) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error while executing query {}: {}", query, e);
                        continue;
                    }
                }
            }
        }
    }
}
