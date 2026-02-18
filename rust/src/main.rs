use eframe::egui;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::io::Result;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};

static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

struct BrowserApp {
    url: String,
    body: Vec<String>,
}

impl Default for BrowserApp {
    fn default() -> Self {
        BrowserApp {
            url: "https://browser.engineering/graphics.html".to_owned(),
            body: vec![
                "<h1>Welcome</h1>".into(),
                "<p>Browser Engineering</p>".into(),
            ],
        }
    }
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("chrome").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Url:");

                if ui.text_edit_singleline(&mut self.url).lost_focus() {
                    println!("Navigating to {}", self.url);
                }

                if ui.button("Refresh").clicked() {
                    self.body.push("New Element".into());
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Render Output");

            let painter = ui.painter();

            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(200.0, 50.0)),
                0.0, // rounding
                egui::Color32::from_rgb(200, 50, 50),
            );
        });
    }
}

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
    // Modified the function to allow to be able to reuse previous connections with the keep alive
    // header
    fn request(
        &self,
        cache: &mut HashMap<String, BufReader<NetworkStream>>,
    ) -> std::io::Result<(BufReader<NetworkStream>, usize)> {
        if self.scheme == "file" {
            let path = &self.path;

            println!("Opening local file: {}", path);
            let file = File::open(path)?;

            let len = file.metadata()?.len() as usize;

            return Ok((BufReader::new(NetworkStream::File(file)), len));
        }

        let mut stream = self.get_connection(cache)?;

        self.send_request(stream.get_mut())?;

        let content_length = self.parse_response_headers(&mut stream)?;

        Ok((stream, content_length))
    }

    fn get_connection(
        &self,
        cache: &mut HashMap<String, BufReader<NetworkStream>>,
    ) -> std::io::Result<BufReader<NetworkStream>> {
        if let Some(reader) = cache.remove(&self.host) {
            return Ok(reader);
        }

        let port = if self.scheme == "https" {
            ":443"
        } else {
            ":80"
        };
        let addr = format!("{}{}", self.host, port);
        let tcp = TcpStream::connect(&addr)?;

        let stream = if self.scheme == "https" {
            let sn = ServerName::try_from(self.host.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            let client =
                ClientConnection::new(get_tls_config(), sn).map_err(std::io::Error::other)?;
            NetworkStream::Tls(Box::new(StreamOwned::new(client, tcp)))
        } else {
            NetworkStream::Plain(tcp)
        };

        Ok(BufReader::new(stream))
    }

    fn send_request(&self, stream: &mut NetworkStream) -> std::io::Result<()> {
        let mut writer = BufWriter::new(stream);
        let path = if self.path.is_empty() {
            "/"
        } else {
            &self.path
        };

        write!(
            writer,
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Connection: keep-alive\r\n\
             User-Agent: RustBrowser/1.0\r\n\
             \r\n",
            path, self.host
        )?;

        writer.flush()
    }
    fn parse_response_headers(
        &self,
        reader: &mut BufReader<NetworkStream>,
    ) -> std::io::Result<usize> {
        let mut line = String::new();

        // Read Status Line (e.g., "HTTP/1.1 200 OK")
        reader.read_line(&mut line)?;

        let mut content_length = 0;

        // Loop through headers
        loop {
            line.clear();
            reader.read_line(&mut line)?;

            // Empty line (\r\n) means end of headers
            if line.trim().is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(':')
                && key.eq_ignore_ascii_case("content-length")
            {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }

        Ok(content_length)
    }
}

fn lex(reader: &mut BufReader<NetworkStream>, len: usize) -> std::io::Result<()> {
    // Translate buffer to exact bytes
    let mut buffer = vec![0u8; len];

    reader.read_exact(&mut buffer)?;

    // Convert bytes to string
    let body = String::from_utf8_lossy(&buffer);
    let cleaned_body = transform_entities(&body);

    println!("HTML Body: {}", cleaned_body);

    Ok(())
}

// Modified function to allow for perssitent connections/sockets (Keep alive)
fn load(url: &Url, cache: &mut HashMap<String, BufReader<NetworkStream>>) -> std::io::Result<()> {
    let (mut reader, content_length) = url.request(cache)?;
    lex(&mut reader, content_length)?;

    // We save the live socket for next time
    cache.insert(url.host.clone(), reader);
    Ok(())
}

fn transform_entities(text: &str) -> String {
    let mut out = String::new();

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '&' {
            // We need to convert the following slice
            let remainder: String = chars[i..].iter().collect();

            if remainder.starts_with("&lt;") {
                out.push('<');
                i += 4;
                continue;
            } else if remainder.starts_with("&gt;") {
                out.push('>');
                i += 4;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn main() -> Result<()> {
    // Earlier previous debug/testing lines
    // let url = Url::new("http://www.google.com/");
    // let request = Url::request(&url);
    // println!("{:?}", url);
    // println!("{:?}\n", request);

    let args: Vec<String> = args().collect();
    let mut connection_cache: HashMap<String, BufReader<NetworkStream>> = HashMap::new();
    let url_str = &args[1];
    let url = Url::new(url_str);

    if args.len() < 2 {
        eprintln!("Usage: {} <url>", args[0]);
        return Ok(());
    }

    // Testing keep alive header
    println!("Initial request");
    load(&url, &mut connection_cache)?;

    println!("Second request");
    load(&url, &mut connection_cache)?;

    Ok(())
}

// Testing the browser application/representation
// fn main() -> eframe::Result<()> {
//     let native_options = eframe::NativeOptions::default();
//     eframe::run_native(
//         "My Rust Browser",
//         native_options,
//         Box::new(|cc| Ok(Box::new(BrowserApp::default()))),
//     )
// }
