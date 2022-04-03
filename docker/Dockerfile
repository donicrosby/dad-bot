ARG RUST_BUILDER
ARG PROD_IMAGE_TAG
ARG S6_OVERLAY_VERSION
FROM ${RUST_BUILDER} as builder
RUN apt-get update && apt-get -y install cmake libssl-dev
WORKDIR build
COPY Cargo.toml .
COPY db/ ./db
COPY bot/ ./bot
RUN cargo build --release --bin dad-bot

FROM ubuntu:${PROD_IMAGE_TAG}
ARG S6_OVERLAY_VERSION
ADD https://github.com/just-containers/s6-overlay/releases/download/${S6_OVERLAY_VERSION}/s6-overlay-amd64-installer /tmp/
RUN chmod +x /tmp/s6-overlay-amd64-installer && /tmp/s6-overlay-amd64-installer /
RUN useradd -ms /bin/bash daduser
RUN apt-get update && apt-get -y install ca-certificates dnsutils iproute2
COPY docker/root/ /
WORKDIR dad_bot
VOLUME /dad_bot/session
VOLUME /dad_bot/store
COPY --from=builder /build/target/release/dad-bot /usr/local/bin
ENV RUST_LOG=INFO
ENTRYPOINT ["/init"]
