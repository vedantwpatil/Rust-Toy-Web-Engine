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

Wrote out some of the introduction code for implementing a basic http client. Able to make basic encrypted requests to websites and display the html content stripped of its tags in a scrollable GUI window. Also added support for local files via the `file://` scheme, persistent connections with keep-alive, and basic HTML entity decoding (`&lt;`, `&gt;`). Extracted tests into their own file to keep things a bit cleaner. The rust implementation has filled out some of the exercises, haven't duplicated my solutions to them in python as it's not the goal of the project.

Added font variant support with bold and italic text are now rendered correctly by tracking `<b>`, `</b>`, `<i>`, `</i>` tags during layout and selecting the appropriate Times New Roman variant (Regular, Bold, Italic, Bold Italic) per word. Each named font family is registered separately in egui since it has no font synthesis, mirroring how the python implementation tracks `weight` and `style` state in its layout function.

### To Review

- Better understand the cryptographic signature and such that was required to implement for secure connections in rust. There was a lot of wrapping the connection with additional parameters which didn't make the most amount of sense
- Look into the word by word stylization, there should be a more efficient way of doing it and with `&str` or `bytes` instead of `String`

### Thoughts on Rust

This is one of my first major projects that I'm taking on in rust from scratch so it's my first experience with some of the more modern features of rust.
Some of the things I've been enjoying so far are

- Matches
  - They feel really intuitive and nice despite other languages having support for cases they feel like they flow better in rust
- The result option type
  - They're really nice to be able to determine which parts of the code can fail and help made error management simple/clean

So far been really enjoying it in comparison to the python implementation because I feel like I know more what is going on. The rust implementation forces me to think more about what I'm doing where in the python solution there's more of a general faith rather than understanding that it'll work
