#!/bin/bash

BUILD_IMAGE_NAME=dad_bot
BUILD_IMAGE_TAG=latest
IMAGE_NAME="${BUILD_IMAGE_NAME}:${BUILD_IMAGE_TAG}"
DOCKER_REPO=donicrosby
PUSH=false

usage() {
  echo "Build/Release dad_bot:"
  echo "    -t: tag to release (required)"
  echo "    -p: push images to upstream"
  echo "    -h: show this message and exit"
}

build_image() {
    COMPOSE_DOCKER_CLI_BUILD=1 DOCKER_BUILDKIT=1 docker-compose build
}

tag_image() {
  docker tag ${IMAGE_NAME} ${DOCKER_REPO}/${BUILD_IMAGE_NAME}:${BUILD_IMAGE_TAG}
  docker tag ${IMAGE_NAME} ${DOCKER_REPO}/${BUILD_IMAGE_NAME}:${TAG}
}

push_image() {
  docker push ${DOCKER_REPO}/${BUILD_IMAGE_NAME}:${TAG}
  docker push ${DOCKER_REPO}/${BUILD_IMAGE_NAME}:${BUILD_IMAGE_TAG}
}


while getopts "t:ph" arg; do
  case $arg in
    h)
      usage
      exit 0
      ;;
    p)
      PUSH=true
      ;;
    t)
      TAG=$OPTARG
      ;;
  esac
done

if [ -z "$TAG" ]; then
  usage
  exit 1
fi

if build_image; then
  tag_image
fi

if $PUSH; then
  push_image
fi
