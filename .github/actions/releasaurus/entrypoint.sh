#!/bin/bash

set -eo pipefail

ARGS=""

if [ "${INPUT_DEBUG}" = "true" ]; then
  ARGS="--debug"
fi

git config --global user.name "${INPUT_GIT_USER_NAME}"
git config --global user.email "${INPUT_GIT_USER_EMAIL}"

releasaurus release \
--github-repo ${INPUT_REPO} \
--github-token ${INPUT_TOKEN} \
--clone-depth ${INPUT_CLONE_DEPTH} ${ARGS}

releasaurus release-pr \
--github-repo ${INPUT_REPO} \
--github-token ${INPUT_TOKEN} \
--clone-depth ${INPUT_CLONE_DEPTH} ${ARGS}
