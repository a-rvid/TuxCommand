use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{self, Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};

#[tokio::main]
async fn main() -> Result<()> {
    // let stream = UnixStream::connect("/tmp/tuxmux.sock").await?;
    // let (mut r, mut w) = stream.into_split();

    // w.write_all to execute
    // socket -> stdout
    // let tx = Arc::new(Mutex::new(w));

    // let mut stdout = io::stdout();
    // let mut buf = [0u8; 1024];
    // tokio::spawn(async move {
    //     loop {
    //         let n = r.read(&mut buf).await.unwrap();
    //         if n == 0 { break; }
    //         let s = String::from_utf8_lossy(&buf[..n]);
    //     }
    // });

    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}
