use dns_server::DnsRecord;
use tokio::fs::{create_dir, exists};
use tokio::thread;
use std::path::Path;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use permit::Permit;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use users::get_current_uid;

fn generate_keypair() -> (RsaPrivateKey, RsaPublicKey) {
    let mut rng = rsa::rand_core::OsRng;
    let keypair = RsaPrivateKey::new(&mut rng, 2048).unwrap(); // 2048 bit key size
    let public_key = keypair.to_public_key();
    (keypair, public_key)
}

const DATA: &str = "/etc/tuxmux";

#[tokio::main]
async fn main() {
    let data = if let Ok(config) = std::env::var("TUXMUX_CONFIG") {
        config
    } else if get_current_uid() == 0 {
        DATA.to_string()
    } else {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| "/tmp".to_string());
        format!("{home}/.tuxmux")
    };

    let port: u16 = std::env::var("TUXMUX_PORT").unwrap_or_else(|_| "53".to_string()).parse().unwrap();

    let data = Path::new(&data);
    if !data.exists() {
        create_dir(data).await.unwrap();
    }

    if !exists(data + "/private.pem").await.unwrap() {
        let (private_key, public_key) = generate_keypair();
        let private_key = private_key.to_pkcs1_pem().unwrap();
        let public_key = public_key.to_pkcs1_pem().unwrap();
        tokio::fs::write(data + "/private.pem", private_key).await.unwrap();
        tokio::fs::write(data + "/public.pem", public_key).await.unwrap();
    }

    let top_permit = Permit::new();
    let permit = top_permit.new_sub();
    std::thread::spawn(move || {
        Signals::new([SIGINT, SIGTERM]).unwrap().forever().next().unwrap();
        drop(top_permit);
    });

    let records = vec![
        DnsRecord::new_txt("example.com", "Hello, world!").unwrap(),
        DnsRecord::new_aaaa("example.com", "::1").unwrap(),
        DnsRecord::new_a("example.com", "0.0.0.0").unwrap(),
        DnsRecord::new_cname("cname.example.com", "example.com").unwrap(),
    ];

    println!(r" /_  __/_  ___  __/  |/  /_  ___  __
  / / / / / / |/_/ /|_/ / / / / |/_/
 / / / /_/ />  </ /  / / /_/ />  <  
/_/  \__,_/_/|_/_/  /_/\__,_/_/|_|  ");
    println!("TuxMux, Config directory: {}", data.display());
    println!("Starting DNS server on port {}", port);

    dns_server::Builder::new_port(port)
        .unwrap()
        .with_permit(permit)
        .serve_static(&records)
        .unwrap();
}