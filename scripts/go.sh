#!/usr/bin/env bash
set -x
set -eo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

sh ${SCRIPT_DIR}/init_db.sh
sh ${SCRIPT_DIR}/init_redis.sh
