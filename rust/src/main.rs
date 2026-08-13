use eframe::egui;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};

static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

// GUI client
struct BrowserApp {
    url: String,
    tokens: Vec<HtmlBody>,
    fonts_loaded: bool,
    connection_cache: HashMap<String, BufReader<NetworkStream>>,
}

impl Default for BrowserApp {
    fn default() -> Self {
        Self {
            url: "https://browser.engineering/".to_owned(),
            tokens: Vec::new(),
            fonts_loaded: false,
            connection_cache: HashMap::new(),
        }
    }
}

impl BrowserApp {
    fn new() -> Self {
        let mut app = Self::default();
        let url = app.url.clone();
        app.navigate(&url);
        app
    }

    fn navigate(&mut self, url_str: &str) {
        let url = match Url::new(url_str) {
            Ok(url) => url,
            Err(e) => {
                self.tokens = vec![HtmlBody::Text(format!("Error: {e}"))];
                return;
            }
        };

        match load(&url, &mut self.connection_cache) {
            Ok(tokens) => self.tokens = tokens,
            Err(e) => self.tokens = vec![HtmlBody::Text(format!("Error: {e}"))],
        }
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
        "TimesNewRomanBold".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Times-Bold.ttf"
        ))),
    );

    fonts.font_data.insert(
        "TimesNewRomanItalic".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Times-Italic.ttf"
        ))),
    );

    fonts.font_data.insert(
        "TimesNewRomanBoldItalic".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Times-BoldItalic.ttf"
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

    for name in [
        "TimesNewRomanBold",
        "TimesNewRomanItalic",
        "TimesNewRomanBoldItalic",
    ] {
        fonts.families.insert(
            egui::FontFamily::Name(name.into()),
            vec![name.to_owned(), "ChineseFontsSupport".to_owned()],
        );
    }

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
                    let url = self.url.clone();
                    self.navigate(&url);
                    println!("Navigating to {}", self.url);
                }

                if ui.button("Refresh").clicked() {
                    let url = self.url.clone();
                    self.navigate(&url);
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
                .auto_shrink([false, false])
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

                    let available_width = ui.available_width();
                    let display_list = layout(&self.tokens, ctx, available_width);

                    let max_y = display_list.iter().map(|d| d.y).fold(0.0_f32, f32::max);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(available_width, max_y + 20.0),
                        egui::Sense::hover(),
                    );

                    let painter = ui.painter();
                    for item in &display_list {
                        painter.text(
                            rect.min + egui::vec2(item.x, item.y),
                            egui::Align2::LEFT_TOP,
                            &item.word,
                            font_id_for(item.bold, item.italic, item.size),
                            egui::Color32::BLACK,
                        );
                    }
                });
        });
    }
}

// Layout

const HSTEP: f32 = 13.0;
const VSTEP: f32 = 18.0;

struct DisplayItem {
    x: f32,
    y: f32,
    word: String,
    bold: bool,
    italic: bool,
    size: f32,
}

fn font_id_for(bold: bool, italic: bool, size: f32) -> egui::FontId {
    let family = match (bold, italic) {
        (true, true) => egui::FontFamily::Name("TimesNewRomanBoldItalic".into()),
        (true, false) => egui::FontFamily::Name("TimesNewRomanBold".into()),
        (false, true) => egui::FontFamily::Name("TimesNewRomanItalic".into()),
        (false, false) => egui::FontFamily::Proportional,
    };
    egui::FontId::new(size, family)
}

fn measure(text: &str, bold: bool, italic: bool, size: f32, ctx: &egui::Context) -> f32 {
    let font_id = font_id_for(bold, italic, size);
    ctx.fonts_mut(|f| text.chars().map(|c| f.glyph_width(&font_id, c)).sum())
}

struct Layout {
    display_list: Vec<DisplayItem>,
    line: Vec<(f32, String, bool, bool, f32)>, // (x, word, bold, italic, size) — y deferred to flush
    cursor_x: f32,
    cursor_y: f32,
    bold: bool,
    italic: bool,
    size: f32,
    width: f32,
}

impl Layout {
    const fn new(width: f32) -> Self {
        Self {
            display_list: Vec::new(),
            line: Vec::new(),
            cursor_x: HSTEP,
            cursor_y: VSTEP,
            bold: false,
            italic: false,
            size: 16.0,
            width,
        }
    }

    fn word(&mut self, word: &str, ctx: &egui::Context) {
        let word_width = measure(word, self.bold, self.italic, self.size, ctx);
        if self.cursor_x + word_width >= self.width - HSTEP {
            self.flush(ctx);
        }
        self.line.push((
            self.cursor_x,
            word.to_string(),
            self.bold,
            self.italic,
            self.size,
        ));
        self.cursor_x += word_width + measure(" ", self.bold, self.italic, self.size, ctx);
    }

    // Two-pass line layout: collect words with x positions, then align on a shared baseline.
    // Needed because mixed font sizes mean y can't be set until we know the tallest word.
    fn flush(&mut self, ctx: &egui::Context) {
        if self.line.is_empty() {
            return;
        }
        let line = std::mem::take(&mut self.line);

        let max_ascent = line
            .iter()
            .map(|(_, _, b, i, s)| ctx.fonts_mut(|f| f.row_height(&font_id_for(*b, *i, *s))) * 0.8)
            .fold(0.0_f32, f32::max);

        let baseline = 1.25f32.mul_add(max_ascent, self.cursor_y);

        let max_descent = line
            .iter()
            .map(|(_, _, b, i, s)| ctx.fonts_mut(|f| f.row_height(&font_id_for(*b, *i, *s))) * 0.2)
            .fold(0.0_f32, f32::max);

        for (x, word, bold, italic, size) in line {
            let ascent = ctx.fonts_mut(|f| f.row_height(&font_id_for(bold, italic, size))) * 0.8;
            self.display_list.push(DisplayItem {
                x,
                y: baseline - ascent,
                word,
                bold,
                italic,
                size,
            });
        }

        self.cursor_y = 1.25f32.mul_add(max_descent, baseline);
        self.cursor_x = HSTEP;
    }

    fn token(&mut self, tok: &HtmlBody, ctx: &egui::Context) {
        match tok {
            HtmlBody::Text(t) => {
                for word in t.split_whitespace() {
                    self.word(word, ctx);
                }
            }
            HtmlBody::Tag(tag) => {
                let tag = tag.trim_matches(|c: char| c == '<' || c == '>').trim();
                match tag {
                    "b" => self.bold = true,
                    "/b" => self.bold = false,
                    "i" => self.italic = true,
                    "/i" => self.italic = false,
                    "small" => self.size -= 2.0,
                    "/small" => self.size += 2.0,
                    "big" => self.size += 4.0,
                    "/big" => self.size -= 4.0,
                    "br" => self.flush(ctx),
                    "/p" => {
                        self.flush(ctx);
                        self.cursor_y += VSTEP;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn layout(tokens: &[HtmlBody], ctx: &egui::Context, width: f32) -> Vec<DisplayItem> {
    let mut l = Layout::new(width);
    for tok in tokens {
        l.token(tok, ctx);
    }
    l.flush(ctx);
    l.display_list
}

// Networking

fn get_tls_config() -> Arc<ClientConfig> {
    TLS_CONFIG
        .get_or_init(|| {
            let root_store = webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned()
                .collect::<RootCertStore>();
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

// Allows us to accept the html body in chunks similar to how websites send them
enum BodyEncoding {
    ContentLength(usize),
    Chunked,
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
    port: Option<u16>,
}

impl Url {
    fn new(input: &str) -> std::io::Result<Self> {
        let (scheme, rest) = input.split_once("://").unwrap_or(("", input));
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let (host, port) = if let Some((h, p)) = host.split_once(':') {
            let port = p.parse::<u16>().map_err(std::io::Error::other)?;
            (h, Some(port))
        } else {
            (host, None)
        };

        Ok(Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            path: format!("/{path}"),
            port,
        })
    }

    // Originally was going to do a one to one converstion but ran into issues with internet protocols so there are slight modifications
    // Modified the function to allow to be able to reuse previous connections with the keep alive
    // header
    fn request(
        &self,
        cache: &mut HashMap<String, BufReader<NetworkStream>>,
    ) -> std::io::Result<(BufReader<NetworkStream>, BodyEncoding)> {
        if self.scheme == "file" {
            let path = &self.path;

            println!("Opening local file: {path}");
            let file = File::open(path)?;

            let len = file.metadata()?.len() as usize;

            return Ok((
                BufReader::new(NetworkStream::File(file)),
                BodyEncoding::ContentLength(len),
            ));
        }

        let mut stream = self.get_connection(cache)?;

        self.send_request(stream.get_mut())?;

        let content_length = parse_response_headers(&mut stream)?;

        Ok((stream, content_length))
    }

    fn get_connection(
        &self,
        cache: &mut HashMap<String, BufReader<NetworkStream>>,
    ) -> std::io::Result<BufReader<NetworkStream>> {
        if let Some(reader) = cache.remove(&self.host) {
            return Ok(reader);
        }

        // Added support for ports in url
        let port = self
            .port
            .unwrap_or(if self.scheme == "https" { 443 } else { 80 });

        let addr = format!("{}:{}", self.host, port);
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
}

fn parse_response_headers(reader: &mut BufReader<NetworkStream>) -> std::io::Result<BodyEncoding> {
    let mut line = String::new();
    let mut is_chunked = false;

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
            && key.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse().map_err(std::io::Error::other)?;

            // Need to parse for transfer encoding headers to be able to properly render
            // webpages, they're a different format
        } else if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("transfer-encoding")
            && value.trim().eq_ignore_ascii_case("chunked")
        {
            is_chunked = true;
        }
    }

    if is_chunked {
        Ok(BodyEncoding::Chunked)
    } else {
        Ok(BodyEncoding::ContentLength(content_length))
    }
}

#[derive(Debug, Clone)]
enum HtmlBody {
    Text(String),
    Tag(String),
}

fn lex(reader: &mut BufReader<NetworkStream>, encoding: BodyEncoding) -> std::io::Result<String> {
    match encoding {
        BodyEncoding::ContentLength(len) => {
            let mut buffer = vec![0u8; len];
            reader.read_exact(&mut buffer)?;
            Ok(String::from_utf8_lossy(&buffer).to_string())
        }
        BodyEncoding::Chunked => {
            let mut body = String::new();
            loop {
                let mut size_line = String::new();
                reader.read_line(&mut size_line)?;

                let chunk_size =
                    usize::from_str_radix(size_line.trim(), 16).map_err(std::io::Error::other)?;

                if chunk_size == 0 {
                    break;
                }

                let mut chunk = vec![0u8; chunk_size];
                reader.read_exact(&mut chunk)?;
                body.push_str(&String::from_utf8_lossy(&chunk));

                let mut crlf = String::new();
                reader.read_line(&mut crlf)?;
            }
            Ok(body)
        }
    }
}

// Modified function to allow for persistent connections/sockets (Keep alive)
fn load(
    url: &Url,
    cache: &mut HashMap<String, BufReader<NetworkStream>>,
) -> std::io::Result<Vec<HtmlBody>> {
    let (mut reader, encoding) = url.request(cache)?;
    let html = lex(&mut reader, encoding)?;

    // We save the live socket for next time
    cache.insert(url.host.clone(), reader);
    Ok(tokenize(&html))
}

fn strip_tags(text: &str) -> Vec<HtmlBody> {
    let mut out: Vec<HtmlBody> = Vec::new();
    let mut buffer = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        if c == '<' {
            if !in_tag && !buffer.is_empty() {
                out.push(HtmlBody::Text(buffer.clone()));
                buffer.clear();
            }
            in_tag = true;
            buffer.push(c);
        } else if c == '>' {
            buffer.push(c);
            out.push(HtmlBody::Tag(buffer.clone()));
            buffer.clear();
            in_tag = false;
        }
        // double newline simulates a paragraph break
        else if c == '\n' && !in_tag {
            buffer.push_str("\n\n");
        } else {
            buffer.push(c);
        }
    }
    if !buffer.is_empty() {
        if in_tag {
            out.push(HtmlBody::Tag(buffer));
        } else {
            out.push(HtmlBody::Text(buffer));
        }
    }
    out
}

fn resolve_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut remainder = text;

    while !remainder.is_empty() {
        if let Some(rest) = remainder.strip_prefix("&lt;") {
            out.push('<');
            remainder = rest;
            continue;
        }
        if let Some(rest) = remainder.strip_prefix("&gt;") {
            out.push('>');
            remainder = rest;
            continue;
        }

        #[allow(clippy::expect_used)]
        // .expect since we've already had a earlier check on the length with the if statement
        let c = remainder
            .chars()
            .next()
            .expect("remainder is non-empty, so it has a first char");
        out.push(c);
        remainder = &remainder[c.len_utf8()..];
    }
    out
}

fn tokenize(html: &str) -> Vec<HtmlBody> {
    strip_tags(html)
        .into_iter()
        .map(|tok| match tok {
            HtmlBody::Text(t) => HtmlBody::Text(resolve_entities(&t)),
            HtmlBody::Tag(t) => HtmlBody::Tag(t),
        })
        .collect()
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
