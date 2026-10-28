//! Server List Ping: the handshake/status exchange the multiplayer menu uses.

use crate::codec::{read_frame, read_string, write_frame, PacketBuilder};
use crate::NextState;
use anyhow::{bail, Context, Result};
use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct StatusResponse {
    /// Raw JSON returned by the server (version, players, description, favicon).
    pub json: serde_json::Value,
    pub latency: Duration,
}

impl StatusResponse {
    pub fn version_name(&self) -> Option<&str> {
        self.json["version"]["name"].as_str()
    }

    pub fn protocol(&self) -> Option<i64> {
        self.json["version"]["protocol"].as_i64()
    }

    pub fn players(&self) -> Option<(i64, i64)> {
        let p = &self.json["players"];
        Some((p["online"].as_i64()?, p["max"].as_i64()?))
    }

    /// The MOTD flattened to plain text (formatting is dropped).
    pub fn motd(&self) -> String {
        fn flatten(v: &serde_json::Value, out: &mut String) {
            match v {
                serde_json::Value::String(s) => out.push_str(s),
                serde_json::Value::Object(o) => {
                    if let Some(t) = o.get("text") {
                        flatten(t, out);
                    }
                    if let Some(extra) = o.get("extra") {
                        flatten(extra, out);
                    }
                }
                serde_json::Value::Array(a) => a.iter().for_each(|e| flatten(e, out)),
                _ => {}
            }
        }
        let mut s = String::new();
        flatten(&self.json["description"], &mut s);
        s
    }
}

/// Pings `host:port` and returns its status. `port` defaults to 25565.
pub fn ping(address: &str, protocol_version: i32) -> Result<StatusResponse> {
    let (host, port) = match address.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().context("invalid port")?),
        None => (address, 25565),
    };
    let addr = (host, port)
        .to_socket_addrs()?
        .next()
        .context("could not resolve address")?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_nodelay(true)?;

    let handshake = PacketBuilder::new(0x00)
        .varint(protocol_version)
        .string(host)
        .u16(port)
        .varint(NextState::Status as i32);
    write_frame(&mut stream, &handshake)?;
    write_frame(&mut stream, &PacketBuilder::new(0x00))?; // Status Request

    let (id, payload) = read_frame(&mut stream)?;
    if id != 0x00 {
        bail!("expected Status Response (0x00), got {id:#04x}");
    }
    let json_str = read_string(&mut &payload[..], 32767)?;
    let json: serde_json::Value = serde_json::from_str(&json_str)?;

    let token = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;
    let sent = Instant::now();
    write_frame(&mut stream, &PacketBuilder::new(0x01).i64(token))?;
    let (id, payload) = read_frame(&mut stream)?;
    let latency = sent.elapsed();
    if id != 0x01 {
        bail!("expected Pong Response (0x01), got {id:#04x}");
    }
    let mut echoed = [0u8; 8];
    (&payload[..]).read_exact(&mut echoed)?;
    if i64::from_be_bytes(echoed) != token {
        bail!("pong token mismatch");
    }

    Ok(StatusResponse { json, latency })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::read_varint;
    use std::net::TcpListener;

    /// A tiny fake server that speaks just enough of the status protocol.
    #[test]
    fn ping_fake_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (id, hs) = read_frame(&mut s).unwrap();
            assert_eq!(id, 0x00);
            let mut r = &hs[..];
            assert_eq!(read_varint(&mut r).unwrap(), 772);
            let (id, _) = read_frame(&mut s).unwrap();
            assert_eq!(id, 0x00);
            let json = r#"{"version":{"name":"1.21.8","protocol":772},
                "players":{"max":20,"online":3},
                "description":{"text":"Hello ","extra":[{"text":"world"}]}}"#;
            write_frame(&mut s, &PacketBuilder::new(0x00).string(json)).unwrap();
            let (id, ping) = read_frame(&mut s).unwrap();
            assert_eq!(id, 0x01);
            let mut pong = PacketBuilder::new(0x01);
            pong = pong.i64(i64::from_be_bytes(ping[..8].try_into().unwrap()));
            write_frame(&mut s, &pong).unwrap();
        });

        let status = ping(&format!("127.0.0.1:{port}"), 772).unwrap();
        server.join().unwrap();
        assert_eq!(status.version_name(), Some("1.21.8"));
        assert_eq!(status.protocol(), Some(772));
        assert_eq!(status.players(), Some((3, 20)));
        assert_eq!(status.motd(), "Hello world");
    }
}
