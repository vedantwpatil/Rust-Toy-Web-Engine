use rustls::StreamOwned;
use rustls::pki_types::ServerName;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env::args;
use std::io::Result;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

type HeaderMap = HashMap<Cow<'static, str>, String>;

// This allows us to treat a Plain TCP stream and a TLS stream as the "same thing"
enum NetworkStream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Read for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for NetworkStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

#[derive(Debug, Clone)]
struct Url {
    scheme: String,
    host: String,
    path: String,
}

impl Url {
    fn new(input: &str) -> Self {
        let (scheme, rest) = input.split_once("://").unwrap_or(("", input));
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));

        Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            path: format!("/{}", path),
        }
    }

    // Originally was going to do a one to one converstion but ran into issues with internet protocols
    // so there are slight modifications

    // TODO:
    // Plan to do another iteration on this where I make the code more rustic
    fn request(url: &Url) -> std::io::Result<BufReader<NetworkStream>> {
        let port = if url.scheme == "https" { ":443" } else { ":80" };
        let address = format!("{}{}", url.host, port);

        let tcp_stream = TcpStream::connect(&address)?;

        // Upgrade to TLS if possible
        let stream = if url.scheme == "https" {
            let root_store =
                rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let rc_config = Arc::new(config);

            let server_name = ServerName::try_from(url.host.clone()).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid DNS Name")
            })?;

            let client = rustls::ClientConnection::new(rc_config, server_name)
                .map_err(std::io::Error::other)?;

            // Wrap the TCP stream in TLS and return the Tls variant
            NetworkStream::Tls(Box::new(StreamOwned::new(client, tcp_stream)))
        } else {
            // Just return the Plain variant
            NetworkStream::Plain(tcp_stream)
        };

        let mut reader = BufReader::new(stream);

        let mut headers: HeaderMap = HashMap::new();
        headers.insert("Host".into(), url.host.clone());
        headers.insert("Connection".into(), "close".to_string());
        headers.insert("User-Agent".into(), "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36".to_string());

        // To upgrade this to 1.1 we need to include a user agent and a connection header
        let mut request = format!(
            "GET {} HTTP/1.0\r\n",
            if url.path.is_empty() { "/" } else { &url.path },
        );

        for (key, value) in headers.iter() {
            request.push_str(key);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }

        request.push_str("\r\n");

        reader.get_mut().write_all(request.as_bytes())?;

        let mut line = String::new();
        reader.read_line(&mut line)?;
        println!("Status: {}", line.trim());
        line.clear();

        loop {
            reader.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            if let Some((key, value)) = line.split_once(":") {
                println!("Key: {}\nValue: {}", key.trim(), value.trim());
            }
            line.clear();
        }

        Ok(reader)
    }
}

fn show(reader: &mut BufReader<NetworkStream>) -> std::io::Result<()> {
    let mut body = String::new();

    reader.read_to_string(&mut body)?;
    println!("HTML Body: {}", body);

    Ok(())
}

fn load(url: &Url) -> std::io::Result<()> {
    let mut reader = Url::request(url)?;
    show(&mut reader)?;

    Ok(())
}
fn main() -> Result<()> {
    // Earlier previous debug/testing lines
    // let url = Url::new("http://www.google.com/");
    // let request = Url::request(&url);
    // println!("{:?}", url);
    // println!("{:?}\n", request);

    let args: Vec<String> = args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <url>", args[0]);
        return Ok(());
    }

    load(&Url::new(&args[1]))?;

    Ok(())
}
