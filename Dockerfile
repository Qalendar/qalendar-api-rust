# Dockerfile (Multi-stage build using clux/muslrust for Alpine/Musl with OpenSSL)

# --- Stage 1: Builder (using clux/muslrust) ---
# This image is designed specifically for cross-compiling Rust to musl targets
# and includes necessary libraries and tools like a musl-compatible C compiler
# and OpenSSL development headers configured for static linking.
# Use the specialized muslrust image
FROM clux/muslrust:latest as builder

# Set the working directory inside the container
WORKDIR /app

# Copy source files - copy Cargo.toml/Cargo.lock first to leverage Docker caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .sqlx ./.sqlx

# Build the application for the x86_64-unknown-linux-musl target in release mode.
# The 'clux/muslrust' image handles the cross-compilation environment and OpenSSL dependencies.
# We specifically use the --target flag to ensure we build for musl.
RUN cargo build --release --target x86_64-unknown-linux-musl

# --- Stage 2: Runner (using Alpine) ---
# Use a minimal Alpine Linux image as the final base for the runtime environment
FROM alpine:latest

# Install runtime dependencies needed by the musl OpenSSL binary (often just ca-certificates for TLS)
# Alpine uses apk package manager
# We use --no-cache to avoid storing package index files
RUN apk update && apk add --no-cache ca-certificates

# Set the working directory inside the container
WORKDIR /app

# Copy the compiled statically-linked binary from the builder stage
# The path inside the builder stage corresponds to where cargo builds for the specific target.
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/qalendar-api /app/qalendar-api

# Ensure the binary is executable (optional, but good practice)
RUN chmod +x /app/qalendar-api

# Expose the port your application listens on (default 8000)
# This is the internal container port. Elastic Beanstalk maps external traffic to this.
EXPOSE 8000

# Set the entrypoint to run your application binary
ENTRYPOINT ["/app/qalendar-api"]

# Environment variables for Elastic Beanstalk should be configured directly on the EB environment.
# Do NOT copy your local .env file into the image.