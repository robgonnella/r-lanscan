FROM rgonnella/releasaurus:v0.2.1

USER root

RUN addgroup -S github && adduser -S github -G github

COPY --chown=github:github entrypoint.sh /entrypoint.sh

USER github

ENTRYPOINT [ "/entrypoint.sh" ]
