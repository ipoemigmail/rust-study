# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------

FROM rust:latest as cargo-build

RUN apt-get update

RUN apt-get install musl-tools -y

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/hello_cargo

COPY Cargo.toml Cargo.toml

RUN mkdir src/

RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

RUN rm -f target/x86_64-unknown-linux-musl/release/deps/hello_cargo*

COPY . .

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

FROM alpine:latest

RUN addgroup -g 1000 hello_cargo

RUN adduser -D -s /bin/sh -u 1000 -G hello_cargo hello_cargo

WORKDIR /home/hello_cargo/bin/

COPY --from=cargo-build /usr/src/hello_cargo/target/x86_64-unknown-linux-musl/release/hello_cargo .

RUN chown hello_cargo:hello_cargo hello_cargo

USER hello_cargo

CMD ["./hello_cargo"]
