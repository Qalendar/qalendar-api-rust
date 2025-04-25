# Stage 1: Build
FROM clux/muslrust:stable as builder

WORKDIR /app
COPY . .

RUN cargo build --release

# Stage 2: Runtime
FROM alpine:latest

# Add SSL certs (for HTTPS support)
RUN apk --no-cache add ca-certificates

# Copy only the built binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qalendar-api /app/qalendar-api

WORKDIR /app

# Run the app
CMD ["./qalendar-api"]
