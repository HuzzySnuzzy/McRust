mod app;
mod camera;
mod renderer;

use anyhow::Result;
use mcrust_protocol::{status, DEFAULT_PROTOCOL_VERSION};

const USAGE: &str = "\
mcrust - a Minecraft: Java Edition compatible client written in Rust

USAGE:
    mcrust                 Open the game window with an offline test world
    mcrust ping <host[:port]>
                           Query a server's status (like the multiplayer menu)
";

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn"),
    )
    .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => app::run(),
        Some("ping") => {
            let addr = args
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("usage: mcrust ping <host[:port]>"))?;
            let s = status::ping(addr, DEFAULT_PROTOCOL_VERSION)?;
            println!(
                "Version : {} (protocol {})",
                s.version_name().unwrap_or("?"),
                s.protocol().unwrap_or(-1)
            );
            if let Some((online, max)) = s.players() {
                println!("Players : {online}/{max}");
            }
            println!("MOTD    : {}", s.motd());
            println!("Latency : {} ms", s.latency.as_millis());
            Ok(())
        }
        Some("-h" | "--help" | "help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => {
            eprint!("unknown command `{other}`\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}
