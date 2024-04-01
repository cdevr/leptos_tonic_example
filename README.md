A rust implementation of a client and server chat.

## Overview

This is a minimal chat client and server example for using [`Leptos`](https://github.com/leptos-rs/leptos) with [`protobufs`](https://protobuf.dev/) using the [`tonic`](https://github.com/hyperium/tonic) crate.
This allows for us to provide powerful gRPC streams to the client that will recieve any messages that hit the server.
Currently the implementation requires the frontend to also generate a version of the `ChatMessage` struct with [`prost`](https://github.com/tokio-rs/prost).
Using `prost`s `Message` trait to convert between the struct implementation and as a byte array. This also requires the
Axum SSR version of [`Leptos`](https://github.com/leptos-rs/start-axum) as there are issues integrating with the `tonic` crate as it has dependencies in [`tokio`](https://github.com/tokio-rs/tokio/) that interferes
with converting to Wasm.

## Getting Started

From the Cargo manifest directory you can setup both the server and client to check this example:

```
cargo r --bin backend
```

Then in a separate terminal:

```
cargo leptos watch
```

Open browser to [`http://localhost:3000`](http://localhost:3000/)

### Rust Version

This example requires `nightly` version of Rust.

To set nightly as a default toolchain for all projects (and add the ability to compile Rust to WebAssembly, if 
you haven’t already):

```
rustup toolchain install nightly
rustup default nightly
rustup target add wasm32-unknown-unknown
```

If you'd like to use `nightly` only in your Leptos project however, add [`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) file with the following content:

```toml
[toolchain]
channel = "nightly"
targets = ["wasm32-unknown-unknown"]
```


`tonic`'s MSRV is `1.70`.

```bash
$ rustup update
$ cargo build
```

### Dependencies

In order to build `tonic` >= 0.8.0, you need the `protoc` Protocol Buffers compiler, along with Protocol Buffers resource files.

#### Ubuntu

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y protobuf-compiler libprotobuf-dev
```

#### Alpine Linux

```sh
sudo apk add protoc protobuf-dev
```

#### Arch Linux

```sh
sudo pacman -S protobuf

```

#### macOS

Assuming [Homebrew](https://brew.sh/) is already installed. (If not, see instructions for installing Homebrew on [the Homebrew website](https://brew.sh/).)

```zsh
brew install protobuf
```

#### Windows

- Download the latest version of `protoc-xx.y-win64.zip` from [HERE](https://github.com/protocolbuffers/protobuf/releases/latest)
- Extract the file `bin\protoc.exe` and put it somewhere in the `PATH`
- Verify installation by opening a command prompt and enter `protoc --version`

### Known Issues

Currently if the browser window is closed the server has no way of being notified to drop the `Sender` from the maintained list of clients.
This doesn't impede any of the other clients from recieving messages. But it isn't ideal. Will have to look into the best way to handle this.
