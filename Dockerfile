FROM rust:1.92.0-alpine3.23

RUN apk add build-base perl openssh lynx ncurses

WORKDIR /workspace

COPY Cargo.lock Cargo.toml ./
COPY lib lib
COPY cli cli
COPY term term

RUN cargo build

ENV TERM=xterm-truecolor
CMD ["tail", "-f", "/dev/null"]
