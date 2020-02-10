# usage (from ..): docker run --rm -it -v "$PWD:/home" [IMAGE]

FROM amazonlinux:2

LABEL org.opencontainers.image.url="https://github.com/chiefbiiko/lambda-wasmtime" \
  org.opencontainers.image.version="0.3.3" \
  org.opencontainers.image.title="lambda-wasmtime-build-image" \
  org.opencontainers.image.description="Image for building the lambda-wasmtime runtime" \
  org.opencontainers.image.authors="Noah Anabiik Schwarz" \
  org.opencontainers.image.licenses="MIT"

ENV RUST_VERSION=1.41.0 \
  CARGO_MAKE_VERSION=0.27.0 \
  PATH=/root/.cargo/bin:$PATH \
  PKG_CONFIG_ALLOW_CROSS=true \
  CARGO_BUILD_TARGET=x86_64-unknown-linux-musl \
  CC_x86_64_unknown_linux_musl=clang \
  CXX_x86_64_unknown_linux_musl=clang++ \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=clang++ \
  RUNTIME_ZIP_FILE_NAME=runtime.zip

RUN yum install -y clang clang-libs clang-devel cmake3 make ncurses-compat-libs ncurses-devel openssl-devel unzip zip && \
  ln -s /usr/bin/cmake3 /usr/bin/cmake && \
  curl -sSf https://sh.rustup.rs | sh -s -- --default-toolchain $RUST_VERSION -y && \
  rustup target add x86_64-unknown-linux-musl && \
  temp_file=$(mktemp) && temp_dir=$(mktemp -d) && \
  curl -fsSL https://github.com/sagiegurari/cargo-make/releases/download/$CARGO_MAKE_VERSION/cargo-make-v$CARGO_MAKE_VERSION-x86_64-unknown-linux-musl.zip -o $temp_file && \
  unzip $temp_file -d $temp_dir && \
  mv $temp_dir/cargo-make-v$CARGO_MAKE_VERSION-x86_64-unknown-linux-musl/cargo-make /root/.cargo/bin/cargo-make && \
  rm $temp_file && rm -rf $temp_dir

VOLUME /home

WORKDIR /home

CMD cargo make --verbose --makefile=build/Makefile.toml runtime
