# web-transcript-parser

Rust crates for parsing TLS connection byte streams and extracting semantic content from committed transcripts. This library enables parsing of TLS messages, decryption of ApplicationData, and extraction of HTTP and JSON content for use in disclosure and proving systems.

## Overview

This project provides tools to:
- Parse byte streams from TLS connections
- Decrypt TLS ApplicationData messages
- Extract and parse HTTP and JSON content from the decrypted data
- Generate range data for selective disclosure of plaintext transcripts
- Support proving systems that generate openings to TLS connection commitments

## Project Structure

This is a Rust workspace containing two crates:

- **`spanner`**: Parsing utilities with span information for tracking byte ranges
- **`context`**: Contextual integrity and transcript parsing for HTTP/TLS connections

## License

MIT License

## Credits

The `spanner` crate was originally forked from [tlsn-utils](https://github.com/tlsnotary/tlsn-utils), and portions of the `context` crate were originally derived from the [tlsn](https://github.com/tlsnotary/tlsn) project's formats crate, pruned and modified for the purposes of this project.
