FROM rust:1.85.1-alpine3.21

RUN apk add alpine-sdk openssl-dev openssl-libs-static openssh

WORKDIR /workspace

COPY Cargo.lock Cargo.toml ./
COPY r-lanlib r-lanlib
COPY r-lanscan r-lanscan
COPY r-lanui r-lanui

RUN cargo build

CMD ["tail", "-f", "/dev/null"]
