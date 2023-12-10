#!/usr/bin/env bash
set -x
set -eo pipefail

RUNNING_CONTAINER=$(docker ps --filter 'name="mailhog' --format '{{.ID}}')

if [[ -n $RUNNING_CONTAINER ]]; then
  echo >&2 "there is a MailHog containter already running, kill it with"
  echo >&2 "    docker kill ${RUNNING_CONTAINTER}"
  exit 1
fi

docker run \
  -p "1025:1025" \
  -p "8025:8025" \
  -d \
  --name "mailhog_$(date '+%s')" \
  mailhog/mailhog

>&2 echo "MailHog is ready to go!"
