SHELL := /bin/bash
.PHONY: all build release

PACKAGE ?= demo_nodes_cpp
CORES := $(shell nproc)

all: build

build:
	npm run tauri dev
release:
	npm run tauri build
