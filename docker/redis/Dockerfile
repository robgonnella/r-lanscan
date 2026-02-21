FROM redis:latest

RUN apt update && apt install -y iproute2

COPY docker/redis/entrypoint.sh /entrypoint.sh

CMD ["/entrypoint.sh"]
