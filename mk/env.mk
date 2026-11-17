# ─── Variables d'environnement & commandes Docker ─────────────────────────────

-include .env
export

MAKEFLAGS += --no-print-directory

DOCKER  ?= /usr/bin/docker
COMPOSE ?= $(DOCKER) compose -f docker-compose.yml

PATH_VOLUME ?= ./postgres_data

COMPOSE_DEV  = $(COMPOSE) -f docker-compose.dev.yml
COMPOSE_PROD = $(COMPOSE) -f docker-compose.prod.yml
COMPOSE_ALL  = $(COMPOSE) -f docker-compose.dev.yml -f docker-compose.prod.yml

DB_USER ?= postgres
DB_NAME ?= $(PROJECT_NAME)_db
BACKUP_DIR = backups

# ─── Couleurs ─────────────────────────────────────────────────────────────────

CYAN   := $(shell printf '\033[36m')
GREEN  := $(shell printf '\033[32m')
YELLOW := $(shell printf '\033[33m')
RED    := $(shell printf '\033[31m')
RESET  := $(shell printf '\033[0m')
