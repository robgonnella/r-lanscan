FROM rust:1.92.0-alpine3.23

RUN apk add build-base perl openssh lynx ncurses iproute2

WORKDIR /workspace

COPY Cargo.lock Cargo.toml ./
COPY lib lib
COPY cli cli
COPY term term

RUN cargo build

COPY docker/workspace/config.yml /root/.config/r-lanterm/config.yml
COPY docker/workspace/entrypoint.sh /entrypoint.sh

ENV TERM=xterm-truecolor

CMD ["/entrypoint.sh"]
