# Rust Toy Web Engine

Toy web engine to learn how browsers are able to interact with each other and how they are able to interpret code to manipulate the webpage.

## Project Structure

Holds both a rust as well as python implementation, the python implementation is the 1:1 translation from the book while the rust implementation is mine.

## Getting Started

You'll need Rust installed [rustup.rs](https://rustup.rs).

```bash
cd rust
cargo run       # launch the browser
cargo test      # run the tests
```

## Current Progress

Wrote out some of the introduction code for implementing a basic http client. Able to make basic encrypted requests to websites and display the html content stripped of its tags in a scrollable GUI window. Also added support for local files via the `file://` scheme, persistent connections with keep-alive, and basic HTML entity decoding (`&lt;`, `&gt;`). Extracted tests into their own file to keep things a bit cleaner. The rust implementation has filled out some of the exercises, haven't duplicated my solutions to them in python as it's not the goal of the project

### Honest Note

Feel somewhat lost on the implementation of securely connecting. I'm able to black box it and understand that we're wrapping our regular request with some additional cryptological secure information to communicate with the server and ensure that both the client and server are the intended parties. The thing which confuses me is the stuff that python abstracted away that aren't there in the rust implementation like the additional crates we brought in as well as the smart pointers.
