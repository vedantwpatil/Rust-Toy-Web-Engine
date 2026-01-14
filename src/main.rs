#[derive(Debug)]
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

fn main() {
    let url = Url::new("http://www.youtube.com/");
    println!("{:?}", url);
}
