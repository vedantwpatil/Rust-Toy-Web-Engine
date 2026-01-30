use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::io::Cursor;
use std::io::Result;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};

type HeaderMap = HashMap<Cow<'static, str>, String>;
static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

fn get_tls_config() -> Arc<ClientConfig> {
    TLS_CONFIG
        .get_or_init(|| {
            let root_store =
                RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

// This allows us to treat a Plain TCP stream and a TLS stream as the "same thing"
enum NetworkStream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<rustls::ClientConnection, TcpStream>>),
    File(std::fs::File),
}

impl Read for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
            Self::File(s) => s.read(buf),
        }
    }
}

impl Write for NetworkStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
            Self::File(_) => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot write to read-only file request",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
            Self::File(s) => s.flush(),
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

    // Originally was going to do a one to one converstion but ran into issues with internet protocols so there are slight modifications
    fn request(&self) -> std::io::Result<BufReader<NetworkStream>> {
        if self.scheme == "file" {
            println!("Opening local file: {}", self.path);
            let file = File::open(&self.path)?;

            return Ok(BufReader::new(NetworkStream::File(file)));
        }

        let port = if self.scheme == "https" {
            ":443"
        } else {
            ":80"
        };
        let address = format!("{}{}", self.host, port);

        let tcp_stream = TcpStream::connect(&address)?;

        // Upgrade to TLS if possible
        let stream = if self.scheme == "https" {
            let server_name = ServerName::try_from(self.host.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

            let client = ClientConnection::new(get_tls_config(), server_name)
                .map_err(std::io::Error::other)?;

            // Wrap the TCP stream in TLS and return the Tls variant
            NetworkStream::Tls(Box::new(StreamOwned::new(client, tcp_stream)))
        } else {
            // Just return the Plain variant
            NetworkStream::Plain(tcp_stream)
        };

        let mut writer = BufWriter::new(stream);

        // "write_fmt" is the streaming version of "format!"
        write!(
            writer,
            "GET {} HTTP/1.1\r\n",
            if self.path.is_empty() {
                "/"
            } else {
                &self.path
            }
        )?;

        // Note: Header is more efficient since we don't need lookups
        let headers: Vec<(Cow<'static, str>, Cow<'_, str>)> = vec![
            ("Host".into(), Cow::Borrowed(&self.host)),
            ("Connection".into(), "close".into()),
            ("User-Agent".into(), "RustBrowser/1.0".into()),
        ];

        for (key, value) in headers {
            write!(writer, "{}: {}\r\n", key, value)?;
        }

        // End of Headers
        write!(writer, "\r\n")?;
        writer.flush()?;

        // READER: Unwrap the stream back out of BufWriter to pass to BufReader
        // (This works because BufWriter::into_inner returns the inner stream)
        let stream = writer.into_inner().map_err(|e| e.into_error())?;
        let mut reader = BufReader::new(stream);

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
