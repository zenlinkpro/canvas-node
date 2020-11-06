FROM ubuntu:18.04 as cargo-build

RUN apt-get update

ENV TZ=Asia/Shanghai

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

RUN apt install -y curl cmake pkg-config libssl-dev git build-essential clang libclang-dev

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y

ENV PATH=/root/.cargo/bin:$PATH

RUN rustup default stable

RUN rustup install nightly-2020-10-06

RUN rustup target add wasm32-unknown-unknown --toolchain nightly-2020-10-06

RUN mkdir canvas-node

COPY ./ canvas-node

RUN  cd canvas-node && \
    cargo build --release

FROM ubuntu:18.04

COPY --from=cargo-build /canvas-node/target/release/canvas /usr/local/bin/canvas

EXPOSE 9944

CMD ["canvas", "--tmp", "--rpc-external", "--ws-external", "--name=canvas-node-test"]