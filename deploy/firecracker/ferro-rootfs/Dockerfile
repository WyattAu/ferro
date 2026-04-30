FROM alpine:3.19 AS rootfs
RUN apk add --no-cache ca-certificates curl
COPY --from=ghcr.io/wyattau/ferro:latest /usr/local/bin/ferro-server /usr/local/bin/ferro-server
COPY --from=ghcr.io/wyattau/ferro:latest /usr/local/bin/ferro-cli /usr/local/bin/ferro-cli
RUN mkdir -p /data && chmod 777 /data
