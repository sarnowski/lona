# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
#
# docker/Makefile - Docker image management for Lona OS build environment

IMAGE_NAME := lona-builder
IMAGE_TAG := latest

UID := $(shell id -u)
GID := $(shell id -g)

.PHONY: build
build: ## Build the Docker image
	docker build \
		--build-arg UID=$(UID) \
		--build-arg GID=$(GID) \
		-t $(IMAGE_NAME):$(IMAGE_TAG) \
		.

.PHONY: rebuild
rebuild: ## Rebuild the Docker image from scratch
	docker build \
		--no-cache \
		--build-arg UID=$(UID) \
		--build-arg GID=$(GID) \
		-t $(IMAGE_NAME):$(IMAGE_TAG) \
		.

.PHONY: clean
clean: ## Remove the Docker image
	docker rmi $(IMAGE_NAME):$(IMAGE_TAG) 2>/dev/null || true

.PHONY: help
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
