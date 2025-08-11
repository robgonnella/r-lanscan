FROM rust:1.89.0-alpine3.21

RUN apk add alpine-sdk openssl-dev openssl-libs-static openssh lynx ncurses

WORKDIR /workspace

COPY Cargo.lock Cargo.toml ./
COPY lib lib
COPY cli cli
COPY term term

RUN cargo build

ENV TERM=xterm-truecolor
CMD ["tail", "-f", "/dev/null"]
