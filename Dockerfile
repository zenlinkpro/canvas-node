FROM zenlinkpro/dex:zenlink_linux_ci as cargo-build

RUN mkdir canvas-node

COPY ./ canvas-node

RUN  cd canvas-node && \
    cargo build --release

FROM ubuntu:18.04

COPY --from=cargo-build /canvas-node/target/release/canvas /usr/local/bin/canvas

EXPOSE 9944

CMD ["canvas", "--tmp", "--rpc-external", "--ws-external", "--name=canvas-node-test"]