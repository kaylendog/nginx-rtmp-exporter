FROM rust:alpine as builder
WORKDIR /build
# install os dependencies
RUN apk add --no-cache build-base openssl-dev
# copy dependency information
COPY Cargo.toml Cargo.lock  ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo fetch && rm -rf src
# copy source and build
COPY build.rs ./
# copy git information
COPY .git .git
COPY src src
RUN cargo build --release

FROM alpine as worker
WORKDIR /app
# install os dependencies
RUN apk add --no-cache openssl-dev tini
# copy executable
COPY --from=builder /build/target/release/nginx-rtmp-exporter ./
# set tini entrypoint and run
ENTRYPOINT [ "/sbin/tini", "--", "/app/nginx-rtmp-exporter" ]

EXPOSE 9114
