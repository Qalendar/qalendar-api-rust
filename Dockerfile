# Stage 1: Build
FROM clux/muslrust:latest AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Final
FROM alpine:latest
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qalendar-api .
RUN chmod +x qalendar-api

EXPOSE 8000
ENTRYPOINT ["./qalendar-api"]