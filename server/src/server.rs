use rusqlite::Connection;
use std::path::Path;
use x25519_dalek::{StaticSecret, SharedSecret, PublicKey};
use tokio::fs;
use std::os::unix::prelude::PermissionsExt;
use users::get_current_uid;

use hickory_proto::op::Message;
use hickory_proto::rr::{RecordType, RData, Record};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// Config file
const DATA: &str = "/etc/tuxmux";

const SPLASH: &str = r" /_  __/_  ___  __/  |/  /_  ___  __
  / / / / / / |/_/ /|_/ / / / / |/_/
 / / / /_/ />  </ /  / / /_/ />  <  
/_/  \__,_/_/|_/_/  /_/\__,_/_/|_|";

struct Keypair {
    private: StaticSecret,
    public: PublicKey,
}

impl Keypair {
    fn generate() -> Self {
        let private = StaticSecret::random();
        let public = PublicKey::from(&private);
        Self {
            private,
            public,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = if let Ok(config) = std::env::var("TUXMUX_CONFIG") {
        config
    } else if get_current_uid() == 0 {
        DATA.to_string()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/.tuxmux")
    };

    let port: u16 = std::env::var("TUXMUX_PORT")
        .unwrap_or_else(|_| "53".to_string())
        .parse()
        .unwrap();

    let data = Path::new(&data);
    if !data.exists() {
        fs::create_dir(data).await.unwrap();
    }

    let conn = init_db(data).await.unwrap();

    // This is the master keypair
    // encrypts everything that the operator can access after a challenge
    let keypair = if fs::try_exists(data.join("private.key")).await.unwrap() {
        let bytes: [u8; 32] = fs::read(data.join("private.key"))
            .await
            .unwrap()
            .try_into()
            .unwrap();
        let private = StaticSecret::from(bytes);
        let public = PublicKey::from(&private);
        Keypair { private, public }
    } else {
        let keypair = Keypair::generate();
        fs::write(data.join("private.key"), &keypair.private.to_bytes()).await.unwrap();
        fs::write(data.join("public.key"), &keypair.public.to_bytes()).await.unwrap();
        fs::set_permissions(data.join("private.key"), PermissionsExt::from_mode(0o600)).await.unwrap();
        fs::set_permissions(data.join("public.key"), PermissionsExt::from_mode(0o600)).await.unwrap();
        keypair
    };
    
    println!("{}", SPLASH); // TUXMUX splash
    println!("Config directory: {}", data.display());

    let socket = UdpSocket::bind(SocketAddr::from_str(format!("0.0.0.0:{}", port).as_str())?).await?;
    println!("Listening on 0.0.0.0:{}", port);
    let cache = Arc::new(Mutex::new(load_from_db().await?));
    let cache_clone = cache.clone();

    // Update loop
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let records = load_from_db().await.unwrap_or_default();
            *cache_clone.lock().await = records;
        }
    });

    // Request loop
    let mut buf = vec![0u8; 4096];
    loop {
        let (len, peer) = socket.recv_from(&mut buf).await?;
        println!("Received request from {}", peer);
        let cache_guard = cache.lock().await;
        let resp = build_response(&buf[..len], &*cache_guard)?;
        socket.send_to(&resp, &peer).await?;
    }
}

fn build_response(req: &[u8], cache: &HashMap<(String, RecordType), String>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req_msg = Message::from_vec(req)?;
    let mut resp = Message::new();
    resp.set_id(req_msg.id());
    resp.set_message_type(hickory_proto::op::MessageType::Response);

    for q in req_msg.queries() {
        resp.add_query(q.clone());

        let mut name = q.name().to_utf8().to_lowercase();
        if name.ends_with('.') { name.pop(); }

        if let Some(ip) = cache.get(&(name, q.query_type())) {
            let rdata = RData::A(hickory_proto::rr::rdata::A(ip.parse::<Ipv4Addr>()?));
            resp.add_answer(Record::from_rdata(q.name().clone(), 300, rdata));
        }
    }
    Ok(resp.to_vec()?)
}

async fn load_from_db() -> Result<HashMap<(String, RecordType), String>, Box<dyn std::error::Error>> {
    let mut records = HashMap::new();
    records.insert(("example.com".to_string(), RecordType::A), "192.168.0.1".to_string());
    Ok(records)
}
async fn init_db(path: &Path) -> rusqlite::Result<Connection, rusqlite::Error> {
    let conn = Connection::open(path.join("tuxmux.db"))?;
    fs::set_permissions(path.join("tuxmux.db"), PermissionsExt::from_mode(0o600)).await.unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    conn.execute_batch(
        "
        BEGIN;

        CREATE TABLE IF NOT EXISTS clients (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT,
            public_key  BLOB NOT NULL,
            private_key BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS status (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id INTEGER NOT NULL,
            heartbeat INTEGER NOT NULL,
            status    INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS commands (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id     INTEGER NOT NULL,
            session_id    TEXT    NOT NULL,
            status        TEXT    NOT NULL DEFAULT 'pending',
            command_queue TEXT    NOT NULL DEFAULT '[]',
            FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS operators (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            auth_key          TEXT    NOT NULL UNIQUE,
            name              TEXT    NOT NULL,
            current_client_id INTEGER,
            FOREIGN KEY (current_client_id) REFERENCES clients(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_status_client   ON status(client_id);
        CREATE INDEX IF NOT EXISTS idx_commands_client ON commands(client_id);
        CREATE INDEX IF NOT EXISTS idx_commands_status ON commands(status);
        CREATE INDEX IF NOT EXISTS idx_operators_key   ON operators(auth_key);

        COMMIT;
    ",
    )?;
    Ok(conn)
}