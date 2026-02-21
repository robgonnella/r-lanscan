FROM localstack/localstack:latest

RUN apt update && apt install -y iproute2

COPY docker/localstack/entrypoint.sh /entrypoint.sh

CMD ["/entrypoint.sh"]
