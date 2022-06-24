FROM rust:alpine as builder
WORKDIR /build
# install os dependencies
RUN apk add --no-cache musl-dev openssl-dev build-base
# copy dependency information
COPY Cargo.toml Cargo.lock  ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo fetch && rm -rf src
# copy source and build
COPY build.rs ./
# copy git information
COPY .git .git
COPY src src
RUN CFLAGS=-mno-outline-atomics cargo build --release

FROM rust:alpine as worker
WORKDIR /app
# install os dependencies
RUN apk add --no-cache tini git libc6-compat
# copy executable
COPY --from=builder /build/target/release/nginx-rtmp-exporter ./
# set tini entrypoint and run
ENTRYPOINT [ "/sbin/tini", "--" ]
CMD [ "/app/nginx-rtmp-exporter" ]

EXPOSE 9114
