#!/bin/bash

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
