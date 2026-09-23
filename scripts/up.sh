#!/bin/bash
set -e

docker run -d \
  --name "${PROJECT_NAME}" \
  -v "$(pwd):/workspaces/${PROJECT_NAME}" \
  -v "${HOME}/.config/gh:/home/user/.config/gh" \
  --add-host=host.docker.internal:host-gateway \
  -e PROJECT_NAME="${PROJECT_NAME}" \
  -e Z_AI_API_KEY="${Z_AI_API_KEY}" \
  "${PROJECT_NAME}:latest" \
  sleep infinity
