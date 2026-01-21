use std::io::{BufRead, BufReader, Read, Write};
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

// Originally was going to do a one to one converstion but ran into issues with internet protocols
// so there are slight modifications
//
// Plan to do a second pass where I make the code more rustic
fn request(url: &Url) -> std::io::Result<TcpStream> {
    let port = ":80";

    let address = url.host.clone() + port;

    let mut stream = TcpStream::connect(address)?;

    let request = format!(
        "GET /ip HTTP/1.0\r\n\
         Host: {}\r\n\
         \r\n",
        url.host
    );

    stream.write_all(request.as_bytes())?;

    let mut reader = BufReader::new(&stream);
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

    let mut response = String::new();

    // Moved the reading of the stream to the bufReader
    reader.read_to_string(&mut response)?;
    println!("{}", response);

    Ok(stream)
}

fn main() {
    let url = Url::new("http://www.google.com/");
    let request = request(&url);
    println!("{:?}", url);
    println!("{:?}", request);
}
