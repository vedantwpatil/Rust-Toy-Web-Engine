use eframe::egui;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};

static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

// Gui client
struct BrowserApp {
    url: String,
    body: String,
    fonts_loaded: bool,
    connection_cache: HashMap<String, BufReader<NetworkStream>>,
}

struct RenderCmd {
    x: f32,
    y: f32,
    text: String,
}

fn layout(ctx: &egui::Context, text: &str, width: f32) -> Vec<RenderCmd> {
    let mut display_list = Vec::new();

    let h_step = 13.0;
    let v_step = 18.0;

    let mut cursor_x = h_step;
    let mut cursor_y = v_step;

    let font_id = egui::FontId::proportional(16.0);

    for word in text.split_whitespace() {
        // Measure the word
        let w = ctx.fonts_mut(|fonts| {
            let galley =
                fonts.layout_no_wrap(word.to_string(), font_id.clone(), egui::Color32::WHITE);
            galley.size().x
        });

        display_list.push(RenderCmd {
            x: cursor_x,
            y: cursor_y,
            text: word.to_string(),
        });

        // Measure a space
        let space_w = ctx.fonts_mut(|fonts| {
            let galley =
                fonts.layout_no_wrap(" ".to_string(), font_id.clone(), egui::Color32::WHITE);
            galley.size().x
        });

        cursor_x += w + space_w;

        // Wrap
        let line_height = ctx.fonts_mut(|fonts| fonts.row_height(&font_id));

        if cursor_x + w >= width - h_step {
            cursor_y += line_height * 1.25;
            cursor_x = h_step;
        }
    }

    display_list
}

impl Default for BrowserApp {
    fn default() -> Self {
        BrowserApp {
            url: "https://browser.engineering/".to_owned(),
            body: String::new(),
            fonts_loaded: false,
            connection_cache: HashMap::new(),
        }
    }
}

impl BrowserApp {
    fn new() -> Self {
        let mut app = BrowserApp::default();
        let url = Url::new(&app.url);
        match load(&url, &mut app.connection_cache) {
            Ok(text) => app.body = text,
            Err(e) => app.body = format!("Error: {}", e),
        }
        app
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "TimesNewRoman".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Times-Regular.ttf"
        ))),
    );

    fonts.font_data.insert(
        "ChineseFontsSupport".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSC.ttf"
        ))),
    );

    let proportional = fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap();

    proportional.insert(0, "ChineseFontsSupport".to_owned());
    proportional.insert(0, "TimesNewRoman".to_owned());

    ctx.set_fonts(fonts);
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.fonts_loaded {
            install_fonts(ctx);
            self.fonts_loaded = true;
        }

        egui::TopBottomPanel::top("chrome").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Url:");

                if ui.text_edit_singleline(&mut self.url).lost_focus() {
                    let url = Url::new(&self.url);

                    match load(&url, &mut self.connection_cache) {
                        Ok(text) => self.body = text,
                        Err(e) => self.body = format!("Error: {}", e),
                    }
                    println!("Navigating to {}", self.url);
                }

                if ui.button("Refresh").clicked() {
                    let url = Url::new(&self.url);

                    match load(&url, &mut self.connection_cache) {
                        Ok(text) => self.body = text,
                        Err(e) => self.body = format!("Error: {}", e),
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut scroll_delta = egui::Vec2::ZERO;

            let input = ctx.input(|i| i.clone());
            if input.key_pressed(egui::Key::ArrowDown) {
                scroll_delta.y -= 40.0;
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                scroll_delta.y += 40.0;
            }
            if input.key_pressed(egui::Key::PageDown) {
                scroll_delta.y -= 300.0;
            }
            if input.key_pressed(egui::Key::PageUp) {
                scroll_delta.y += 300.0;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false]) // don’t auto-shrink width/height
                .id_salt("main_scroll")
                .show(ui, |ui| {
                    // TODO: Fix
                    // Want to scroll with cursor, doesn't work currently
                    //
                    // if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                    //     ui.scroll_to_cursor(Some(egui::Align::TOP));
                    // }
                    //
                    // if ctx.input(|i| i.key_pressed(egui::Key::End)) {
                    //     ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    // }

                    if scroll_delta != egui::Vec2::ZERO {
                        ui.scroll_with_delta(scroll_delta);
                    }
                    ui.set_min_width(ui.available_width());
                    ui.label(&self.body);
                });
        });
    }
}

// Networking

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

fn lex(reader: &mut BufReader<NetworkStream>, len: usize) -> std::io::Result<String> {
    // Translate buffer to exact bytes
    let mut buffer = vec![0u8; len];

    reader.read_exact(&mut buffer)?;

    // Convert bytes to string
    let body = String::from_utf8_lossy(&buffer);
    Ok(transform_entities(&body))
}

// Modified function to allow for perssitent connections/sockets (Keep alive)
fn load(
    url: &Url,
    cache: &mut HashMap<String, BufReader<NetworkStream>>,
) -> std::io::Result<String> {
    let (mut reader, content_length) = url.request(cache)?;
    let text = lex(&mut reader, content_length)?;

    // We save the live socket for next time
    cache.insert(url.host.clone(), reader);
    Ok(text)
}

fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        }
        // double newline simulates a paragraph break
        else if c == '\n' {
            out.push_str("\n\n");
        } else if !in_tag {
            out.push(c);
        }
    }

    out
}

fn transform_entities(text: &str) -> String {
    let stripped = strip_tags(text);

    let chars: Vec<char> = stripped.chars().collect();
    let mut out = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Handle entities
        if c == '&' {
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
        out.push(c);
        i += 1;
    }
    out
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "My Rust Browser",
        native_options,
        Box::new(|_cc| Ok(Box::new(BrowserApp::new()))),
    )
}

#[cfg(test)]
mod tests;
