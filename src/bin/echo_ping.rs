use std::{net::SocketAddr, path::Path, str::FromStr};

use tokio::time::Instant;

#[tokio::main]
async fn main() {
    let mut args = std::env::args();

    let program_name = args.nth(0).unwrap();
    let program_name = Path::new(&program_name).file_stem().unwrap();
    let program_name = program_name.to_str().unwrap();

    let print_help = || println!("Useage: \n\t{} <ip:port>", program_name);

    if args.len() != 1 {
        print_help();
        return;
    }

    let target = match SocketAddr::from_str(&args.nth(0).unwrap()) {
        Ok(target) => target,
        Err(e) => {
            println!("address invalid {}", e);
            print_help();
            return;
        }
    };

    let client = echo_client::Client::new().await;

    loop {
        let begin = Instant::now();

        let res = client.ping_once(&target).await;

        let end = Instant::now();
        let elapsed = end.duration_since(begin);
        println!("{} {} {}ms", target, res, elapsed.as_millis());
    }
}
