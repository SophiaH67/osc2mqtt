# docker run -v $PWD:/volume --rm -t clux/muslrust:stable cargo build --release
FROM clux/muslrust:stable AS builder
# Copy source code
RUN cargo new --bin osc2mqtt
WORKDIR /osc2mqtt
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
# Build
RUN cargo build --release --target=x86_64-unknown-linux-musl

# Release
FROM scratch
COPY --from=builder /osc2mqtt/target/x86_64-unknown-linux-musl/release/osc2mqtt /osc2mqtt
CMD ["/osc2mqtt"]