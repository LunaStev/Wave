#!/bin/bash

# This file is part of the Wave language project.
# Copyright (c) 2024–2026 Wave Foundation
# Copyright (c) 2024–2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

IMAGE_NAME="wave-dev"
CONTAINER_NAME="wave-dev-container"
DOCKERFILE_NAME="Dockerfile"

if [[ "$(docker images -q $IMAGE_NAME 2> /dev/null)" == "" ]]; then
  echo "🔧 Docker image not found. Building image..."
  docker build -t $IMAGE_NAME -f $DOCKERFILE_NAME .
else
  echo "✅ Docker image already exists."
fi

if [[ "$(docker ps -aq -f name=$CONTAINER_NAME)" == "" ]]; then
  echo "🚀 Starting new container..."
  docker run -it --name $CONTAINER_NAME -v $(pwd):/wave $IMAGE_NAME
else
  echo "♻️  Container already exists."

  if [[ "$(docker inspect -f '{{.State.Running}}' $CONTAINER_NAME)" == "true" ]]; then
    echo "🔄 Attaching to running container..."
    docker exec -it $CONTAINER_NAME /bin/bash
  else
    echo "▶️ Starting and attaching to container..."
    docker start $CONTAINER_NAME
    docker exec -it $CONTAINER_NAME /bin/bash
  fi
fi
