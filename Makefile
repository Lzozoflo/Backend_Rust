.PHONY: all help dev prod logs logs-% prod-logs prod-logs-% ps restart \
        db-export db-import db-exportimport-delete db-shell down down-v prune rmi volumes clean fclean re in-% \
        unit unit-local test-all

# ─── Environnement & Variables ────────────────────────────────────────────────

include mk/env.mk

POSTGRES_CONTAINER ?= ${PROJECT_NAME:?copie .env.example vers .env}_db
POSTGRES_DB        ?= ${PROJECT_NAME:?copie .env.example vers .env}_db
POSTGRES_USER      ?= postgres

PATH_DATABASE_VOLUME ?= $(HOME)/${PROJECT_NAME:?copie .env.example vers .env}_db_data
BACKUP_FILE  = $(HOME)/backup_${PROJECT_NAME:?copie .env.example vers .env}_db.sql

# ─── Aide par défaut ──────────────────────────────────────────────────────────

all: help

help:
	@echo "$(CYAN)═══ Commandes Makefile ═══$(RESET)"
	@echo "  $(GREEN)make dev$(RESET)          : Lance l'environnement de DEV (Hot-Reload)"
	@echo "  $(GREEN)make prod$(RESET)         : Build et lance l'environnement de PROD"
	@echo "  $(GREEN)make ps$(RESET)           : Affiche l'état des conteneurs"
	@echo "  $(GREEN)make logs$(RESET)         : Suit les logs en DEV (ou logs-backend, logs-db)"
	@echo "  $(GREEN)make down$(RESET)         : Coupe les conteneurs (GARDE la DB)"
	@echo "  $(CYAN)make unit$(RESET)         : Tests unitaires dans Docker (cargo test --lib)"
	@echo "  $(CYAN)make unit-local$(RESET)   : Tests unitaires avec ton Cargo local"
	@echo "  $(CYAN)make test-all$(RESET)     : Unitaires + tests d'intégration (sans base)"
	@echo "  $(CYAN)make test-db$(RESET)      : Parcours complet contre une vraie base Postgres"
	@echo "  $(YELLOW)make db-export$(RESET)    : Sauvegarde la DB dans $(BACKUP_FILE)"
	@echo "  $(YELLOW)make db-import$(RESET)    : Restaure la DB depuis $(BACKUP_FILE)"
	@echo "  $(YELLOW)make db-shell$(RESET)     : Ouvre un terminal SQL (psql) dans la DB"
	@echo "  $(RED)make down-v$(RESET)       : Coupe les conteneurs et détruit les volumes Docker"
	@echo "  $(RED)make re$(RESET)           : Nettoie Docker (fclean) et relance le DEV"

# ─── MODE DÉVELOPPEMENT (Hot-Reload) ──────────────────────────────────────────

dev:
	@mkdir -p $(PATH_DATABASE_VOLUME)
	$(COMPOSE_DEV) up -d
	@echo "$(GREEN)🚀 Environnement DEV lancé ! (DB stockée dans $(PATH_DATABASE_VOLUME))$(RESET)"

logs:
	$(COMPOSE_DEV) logs -f

logs-%:
	$(COMPOSE_DEV) logs -f $*

restart:
	$(COMPOSE_DEV) restart

# ─── MODE PRODUCTION (binaire Rust, image runtime légère) ─────────────────────

prod:
	@mkdir -p $(PATH_DATABASE_VOLUME)
	$(COMPOSE_PROD) up -d --build
	@echo "$(GREEN)🚀 Environnement PROD lancé !$(RESET)"

prod-logs:
	$(COMPOSE_PROD) logs -f

prod-logs-%:
	$(COMPOSE_PROD) logs -f $*

# ─── SAUVEGARDE, TRANSFERT & OUTILS DB ────────────────────────────────────────

db-export:
	@$(DOCKER) exec -t $(POSTGRES_CONTAINER) pg_dump -U $(POSTGRES_USER) -d $(POSTGRES_DB) --clean --if-exists > $(BACKUP_FILE)
	@echo "$(GREEN)✅ Base sauvegardée dans $(BACKUP_FILE)$(RESET)"

db-import:
	@if [ ! -f $(BACKUP_FILE) ]; then \
		echo "$(RED)❌ Fichier $(BACKUP_FILE) introuvable !$(RESET)"; exit 1; \
	fi
	@cat $(BACKUP_FILE) | $(DOCKER) exec -i $(POSTGRES_CONTAINER) psql -U $(POSTGRES_USER) -d $(POSTGRES_DB) >/dev/null
	@echo "$(GREEN)✅ Base restaurée avec succès depuis $(BACKUP_FILE)$(RESET)"

db-exportimport-delete:
	rm $(BACKUP_FILE)

db-shell:
	$(DOCKER) exec -it $(POSTGRES_CONTAINER) psql -U $(POSTGRES_USER) -d $(POSTGRES_DB)

# ─── ARRÊT ET NETTOYAGE ───────────────────────────────────────────────────────

# Coupe les conteneurs MAIS garde les données de la DB
down:
	$(COMPOSE_ALL) down
	@echo "$(YELLOW)🛑 Conteneurs arrêtés (données conservées).$(RESET)"

# Coupe les conteneurs ET détache les volumes nommés
down-v:
	$(COMPOSE_ALL) down -v
	@echo "$(RED)🗑️  Conteneurs et volumes Docker supprimés (le dossier $(PATH_DATABASE_VOLUME) reste intact sur l'hôte).$(RESET)"

prune:
	$(DOCKER) system prune -af --volumes

rmi:
	$(DOCKER) container rm -f $$($(DOCKER) ps -aq) 2>/dev/null || true
	$(DOCKER) rmi -f $$($(DOCKER) images -q) 2>/dev/null || true

volumes:
	$(DOCKER) volume rm $$($(DOCKER) volume ls -q) 2>/dev/null || true

clean:
	$(MAKE) prune
	$(MAKE) rmi

fclean: clean
	$(DOCKER) volume prune -f
	$(DOCKER) network prune -f
	$(MAKE) volumes

re: fclean
	$(MAKE) dev

# ─── Logs & Debug ─────────────────────────────────────────────────────────────

ps:
	$(DOCKER) ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

in-%:
	$(COMPOSE_DEV) exec $* sh

# ─── Tests ────────────────────────────────────────────────────────────────────

# Tests unitaires (blocs UNIT TEST dans src/), dans Docker
unit:
	$(DOCKER) run --rm \
		-v $(CURDIR):/app \
		-v cargo-registry:/usr/local/cargo/registry \
		-v cargo-target:/app/target \
		-w /app \
		$(RUST_IMAGE) \
		cargo test --lib

# Pareil avec le Cargo installé sur ta machine (plus rapide)
unit-local:
	cargo test --lib

# Unitaires + tests d'intégration sans base (dossier tests/)
test-all:
	cargo test


rename:
	sed -i '0,/^name = ".*"/s//name = "$(PROJECT_NAME)"/' Cargo.toml


	
include mk/test.mk