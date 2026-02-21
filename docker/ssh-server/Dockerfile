FROM lscr.io/linuxserver/openssh-server:latest

RUN apk add iproute2

COPY docker/ssh-server/network-delay.sh /custom-cont-init.d/99-network-delay.sh
