FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/qalendar-api ./qalendar-api
RUN chmod +x qalendar-api

EXPOSE 8000
ENTRYPOINT ["./qalendar-api"]
