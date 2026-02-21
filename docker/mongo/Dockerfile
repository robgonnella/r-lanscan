FROM mongo:latest

RUN apt update && apt install -y iproute2

COPY docker/mongo/entrypoint.sh /entrypoint.sh

CMD ["/entrypoint.sh"]
