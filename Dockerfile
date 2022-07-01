FROM rust as builder
WORKDIR /build
# install os dependencies
RUN apt update
RUN apt install -y libssl-dev build-essential
# copy dependency information
COPY Cargo.toml Cargo.lock  ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo fetch && rm -rf src
# copy source and build
COPY build.rs ./
# copy git information
COPY .git .git
COPY src src
RUN cargo build --release

FROM ubuntu as worker
WORKDIR /app
# install os dependencies
RUN apt install -y libssl-dev
# install tini
ENV TINI_VERSION v0.19.0
RUN apt update
RUN apt install -y git
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini
# copy executable
COPY --from=builder /build/target/release/nginx-rtmp-exporter ./
# set tini entrypoint and run
ENTRYPOINT [ "/tini", "--", "/app/nginx-rtmp-exporter" ]

EXPOSE 9114
