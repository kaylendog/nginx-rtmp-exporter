FROM rust as builder
WORKDIR /build
# copy dependency information
COPY Cargo.toml Cargo.lock  ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo fetch && rm -rf src
# copy source and build
COPY build.rs ./
# copy git information
COPY .git .git
COPY src src
RUN cargo build --release

FROM debian:12-slim as worker
WORKDIR /app
# install os dependencies
RUN apt-get update && apt-get install -y libssl3
# add tini
ENV TINI_VERSION v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
# copy executable
COPY --from=builder /build/target/release/nginx-rtmp-exporter ./
# set tini entrypoint and run
ENTRYPOINT [ "/tini", "--", "/app/nginx-rtmp-exporter" ]

EXPOSE 9114
