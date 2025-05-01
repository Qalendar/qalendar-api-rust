FROM alpine:3.18
RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY target/release/qalendar-api ./qalendar-api
RUN chmod +x qalendar-api

EXPOSE 8000
ENTRYPOINT ["./qalendar-api"]
