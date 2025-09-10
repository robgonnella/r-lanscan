FROM rgonnella/releasaurus:v0.2.1

USER root

RUN mkdir -p /github/home \
  && addgroup -S github && adduser -h /github/home -S github -G github \
  && chown -R github:github /github

COPY --chown=github:github entrypoint.sh /entrypoint.sh

USER github

ENTRYPOINT [ "/entrypoint.sh" ]
