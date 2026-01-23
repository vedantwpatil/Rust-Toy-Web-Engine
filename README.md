# Rust Toy Web Engine

Toy web engine to learn how browsers are able to interact with each other and how they are able to interpret code to manipulate the webpage.

## Project Structure

Holds both a rust as well as python implementation, the python implementation is the 1:1 translation from the book while the rust implementation is mine.

## Current Progress

Wrote out some of the introduction code for implementing a basic http client. Able to make basic encrypted requests to websites and display the request header information in addition to the html content on the webpage.

### Honest Note

Feel somewhat lost on the implementation of securely connecting. I'm able to black box it and understand that we're wrapping our regular request with some additional cryptological secure information to communicate with the server and ensure that both the client and server are the intended parties. The thing which confuses me is the stuff that python abstracted away that aren't there in the rust implementation like the additional crates we brought in as well as the smart pointers.
