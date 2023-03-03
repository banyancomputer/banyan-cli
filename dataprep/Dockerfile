# TODO: docker build for Dataprep cli
## syntax=docker/dockerfile:1
#ARG RUST_BUILD_IMG=rust:1.65-slim-bullseye
#ARG DEBIAN_TAG=bullseye-slim
#
#FROM $RUST_BUILD_IMG as base
#
## AMD64
#FROM --platform=$BUILDPLATFORM base as builder-amd64
#ARG TARGET="x86_64-unknown-linux-gnu"
#
## ARM64
#FROM --platform=$BUILDPLATFORM base as builder-arm64
#ARG TARGET="aarch64-unknown-linux-gnu"
#
#FROM builder-$TARGETARCH as builder
#
#RUN adduser --disabled-password --disabled-login --gecos "" --no-create-home dataprep
#RUN apt update && apt install -y g++
#RUN rustup target add $TARGET
#
#RUN cargo init dataprep
#
#WORKDIR /dataprep
#
## touch lib.rs as we combine both
#Run touch src/lib.rs
#
## touch benches as it's part of Cargo.toml
#RUN mkdir benches
#RUN touch benches/a_benchmark.rs
#
## copy cargo.*
#COPY ../Cargo.lock ./Cargo.lock
#COPY ../Cargo.toml ./Cargo.toml
#
## cache depencies
#RUN mkdir .cargo
#RUN cargo vendor > .cargo/config
#RUN --mount=type=cache,target=$CARGO_HOME/registry \
#    --mount=type=cache,target=$CARGO_HOME/.git \
#    --mount=type=cache,target=dataprep/target,sharing=locked \
#    cargo build --target $TARGET --release
#
#COPY src ./src
## copy src
#COPY src ./src
## copy benches
#COPY benches ./benches
#
## final build for release
#RUN rm ./target/$TARGET/release/deps/*dataprep*
#RUN --mount=type=cache,target=$CARGO_HOME/registry \
#    --mount=type=cache,target=$CARGO_HOME/.git \
#    --mount=type=cache,target=dataprep/target,sharing=locked \
#    cargo build --target $TARGET --bin dataprep --release
#
#RUN strip ./target/$TARGET/release/dataprep
#RUN mv ./target/$TARGET/release/dataprep* /usr/local/bin
#
#FROM debian:${DEBIAN_TAG}
#
#ARG backtrace=0
#ARG log_level=info
#
#ENV RUST_BACKTRACE=${backtrace} \
#    RUST_LOG=${log_level}
#
#COPY --from=builder /usr/local/bin/dataprep* .
#COPY --from=builder /etc/passwd /etc/passwd
#COPY --from=builder /etc/group /etc/group
#
#USER dataprep:dataprep
#ENTRYPOINT ["./dataprep"]
