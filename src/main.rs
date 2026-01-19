use std::net::TcpStream;

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
        let parsed = Self {
            scheme: scheme.to_string(),
            host: host.to_string(),
            path: format!("/{}", path),
        };

        assert_eq!(parsed.scheme, "http");
        parsed
    }
}

// Originally was going to do a one to one converstion but ran into issues with iternet protocols
fn request(url: &Url) -> std::io::Result<TcpStream> {
    let port = ":80";

    let address = url.host.clone() + port;

    let stream = TcpStream::connect(address)?;

    Ok(stream)
}

fn main() {
    let url = Url::new("http://www.youtube.com/");
    let request = request(&url);
    println!("{:?}", url);
    println!("{:?}", request);
}
