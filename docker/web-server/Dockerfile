FROM nginx:latest

RUN apt update && apt install -y iproute2

COPY docker/web-server/entrypoint.sh /entrypoint.sh

CMD ["/entrypoint.sh"]
