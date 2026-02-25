# Rust Toy Web Engine

Toy web engine to learn how browsers are able to interact with each other and how they are able to interpret code to manipulate the webpage.

## Project Structure

Holds both a rust as well as python implementation, the python implementation is the 1:1 translation from the book while the rust implementation is mine. To mimic the python structure of the project initially just going to have everything in one massive `main.rs` file and then will separate the project later.

## Getting Started

You'll need Rust installed [rustup.rs](https://rustup.rs).

```bash
cd rust
cargo run       # launch the browser
cargo test      # run the tests
```

## Current Progress

Wrote out some of the introduction code for implementing a basic http client. Able to make basic encrypted requests to websites and display the html content stripped of its tags in a scrollable GUI window. Also added support for local files via the `file://` scheme, persistent connections with keep-alive, and basic HTML entity decoding (`&lt;`, `&gt;`). Extracted tests into their own file to keep things a bit cleaner. The rust implementation has filled out some of the exercises, haven't duplicated my solutions to them in python as it's not the goal of the project

### To Review

Better understand the cryptographic signature and such that was required to implement for secure connections in rust. There was a lot of wrapping the connection with additional parameters which didn't make the most amount of sense
