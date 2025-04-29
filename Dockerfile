# Dockerfile (Multi-Stage for Alpine/Musl Cross-Compilation)

# --- Build Stage ---
# Use a base image that has the Rust compiler and Musl toolchain
# rust:latest-slim-bullseye includes the standard Rust toolchain
FROM rust:slim-bullseye as builder

# Install musl-tools for cross-compilation to x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

# Set the working directory inside the builder container
WORKDIR /app

# Copy your source code into the builder stage
COPY . .

# Build the Rust application for the musl target
# Use --locked to ensure repeatable builds if you use Cargo.lock
# Use --features <your_features> if you have any cargo features enabled
RUN cargo build --release --target x86_64-unknown-linux-musl --locked

# --- Final Stage ---
# Use a minimal Alpine Linux image
FROM alpine:latest

# Set the working directory inside the final container
WORKDIR /app

# Install necessary runtime dependencies (often just ca-certificates for TLS)
# Rust's native-tls might use the system's CA store.
RUN apk update && apk add --no-cache ca-certificates

# Copy the compiled binary from the builder stage
# The path is relative to the builder's WORKDIR (/app)
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qalendar-api /app/qalendar-api

# Ensure the binary is executable
RUN chmod +x /app/qalendar-api

# Expose the port your application listens on
EXPOSE 8000

# Set the entrypoint to run your application binary
ENTRYPOINT ["/app/qalendar-api"]

# Environment variables should be configured directly on the Elastic Beanstalk environment.
# Do NOT copy your local .env file into the image.