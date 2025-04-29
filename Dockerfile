# Dockerfile
# Use Alpine Linux, a very small base image suitable for static binaries
FROM alpine:latest

# Set the working directory inside the container
WORKDIR /app

# Copy the locally built MUSL release binary into the container
# Assumes you have built the binary using:
# cargo build --release --target x86_64-unknown-linux-musl
# The path is relative to the root of the cargo project.
COPY target/x86_64-unknown-linux-musl/release/qalendar-api /app/qalendar-api

# Ensure the binary is executable (optional, but good practice)
RUN chmod +x /app/qalendar-server

# Expose the port your application listens on (default 8000)
# This is the internal container port. Elastic Beanstalk maps external traffic to this.
EXPOSE 8000

# Set the entrypoint to run your application binary
ENTRYPOINT ["/app/qalendar-api"]

# Environment variables should be configured directly on the Elastic Beanstalk environment.
# Do NOT copy your local .env file into the image.