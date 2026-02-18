use super::*;

// --- Url::new parsing ---

#[test]
fn test_url_parses_http() {
    let url = Url::new("http://www.google.com/");
    assert_eq!(url.scheme, "http");
    assert_eq!(url.host, "www.google.com");
    assert_eq!(url.path, "/");
}

#[test]
fn test_url_parses_https() {
    let url = Url::new("https://browser.engineering/");
    assert_eq!(url.scheme, "https");
    assert_eq!(url.host, "browser.engineering");
    assert_eq!(url.path, "/");
}

#[test]
fn test_url_parses_path() {
    let url = Url::new("https://example.com/foo/bar");
    assert_eq!(url.scheme, "https");
    assert_eq!(url.host, "example.com");
    assert_eq!(url.path, "/foo/bar");
}

#[test]
fn test_url_parses_file_scheme() {
    let url = Url::new("file:///tmp/test.html");
    assert_eq!(url.scheme, "file");
    assert_eq!(url.host, "");
    assert_eq!(url.path, "/tmp/test.html");
}

// --- strip_tags ---

#[test]
fn test_strip_tags_removes_tags() {
    assert_eq!(strip_tags("<h1>Hello</h1>"), "Hello");
}

#[test]
fn test_strip_tags_no_tags() {
    assert_eq!(strip_tags("plain text"), "plain text");
}

#[test]
fn test_strip_tags_nested() {
    assert_eq!(strip_tags("<div><p>text</p></div>"), "text");
}

#[test]
fn test_strip_tags_empty() {
    assert_eq!(strip_tags(""), "");
}

// --- transform_entities ---

#[test]
fn test_transform_entities_lt() {
    assert_eq!(transform_entities("a &lt; b"), "a < b");
}

#[test]
fn test_transform_entities_gt() {
    assert_eq!(transform_entities("a &gt; b"), "a > b");
}

#[test]
fn test_transform_entities_both() {
    assert_eq!(transform_entities("&lt;tag&gt;"), "<tag>");
}

#[test]
fn test_transform_entities_strips_tags_first() {
    assert_eq!(transform_entities("<b>bold &lt;text&gt;</b>"), "bold <text>");
}

#[test]
fn test_transform_entities_no_entities() {
    assert_eq!(transform_entities("hello world"), "hello world");
}
