# FROM amazonlinux:2
FROM lambci/lambda:build-provided

LABEL org.opencontainers.image.url="https://github.com/chiefbiiko/lambda-wasmtime" \
  org.opencontainers.image.version="0" \
  org.opencontainers.image.title="lambda-wasmtime-build-image" \
  org.opencontainers.image.description="Image for building the lambda-wasmtime runtime" \
  org.opencontainers.image.authors="Noah Anabiik Schwarz" \
  org.opencontainers.image.licenses="MIT"

ENV RUST_VERSION=1.41.0

RUN curl https://sh.rustup.rs -sSf \
  | sh -s -- --default-toolchain ${RUST_VERSION} -y

ENV PATH=/root/.cargo/bin:$PATH

RUN yum erase -y cmake

RUN yum install -y cmake3

RUN ln -s /usr/bin/cmake3 /usr/bin/cmake

RUN cargo install cargo-make

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /var/task

VOLUME /var/task

CMD cargo make --makefile build/Makefile.toml --verbose runtime
