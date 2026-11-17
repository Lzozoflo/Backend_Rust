.PHONY: all help dev prod logs logs-% prod-logs prod-logs-% ps restart \
        db-export db-import db-shell down clean fclean re in-% \
        unit unit-docker test-all rename \
        migration migration-empty migrate revert redo schema

# ─── Environnement & Variables ────────────────────────────────────────────────

include mk/env.mk
include mk/diesel.mk
include mk/test.mk


# ─── Aide par défaut ──────────────────────────────────────────────────────────

all: help

help:
	@echo "$(CYAN)═══ Commandes Makefile ═══$(RESET)"
	@echo "  $(GREEN)make dev$(RESET)          : Lance l'environnement de DEV (Hot-Reload)"
	@echo "  $(GREEN)make prod$(RESET)         : Build et lance l'environnement de PROD"
	@echo "  $(GREEN)make ps$(RESET)           : Affiche l'état des conteneurs"
	@echo "  $(GREEN)make logs$(RESET)         : Suit les logs en DEV (ou logs-backend, logs-db)"
	@echo "  $(CYAN)make unit$(RESET)         : Tests unitaires dans Docker (cargo test --lib)"
	@echo "  $(CYAN)make unit-local$(RESET)   : Tests unitaires avec ton Cargo local"
	@echo "  $(CYAN)make test-all$(RESET)     : Unitaires + tests d'intégration (sans base)"
	@echo "  $(CYAN)make test-db$(RESET)      : Parcours complet contre une vraie base Postgres"
	@echo "  $(CYAN)make migration name=xxx$(RESET) : GÉNÈRE la migration depuis src/schema.rs (diesel --diff-schema)"
	@echo "  $(CYAN)make migrate$(RESET)      : Applique les migrations (et réécrit src/schema.rs)"
	@echo "  $(CYAN)make revert$(RESET)       : Annule la dernière migration"
	@echo "  $(YELLOW)make db-shell$(RESET)     : Ouvre un terminal SQL (psql) dans la DB"
	@echo "  $(YELLOW)make db-export$(RESET)    : Sauvegarde la DB dans $(BACKUP_FILE)"
	@echo "  $(YELLOW)make db-import$(RESET)    : Restaure la DB depuis $(BACKUP_FILE)"
	@echo "  $(YELLOW)make down$(RESET)         : Arrête les conteneurs sans rien supprimer"
	@echo "  $(RED)make clean$(RESET)        : Arrête les conteneurs et supprime les images"
	@echo "  $(RED)make fclean$(RESET)       : Supprime images, volumes, données DB et prune Docker"
	@echo "  $(RED)make re$(RESET)           : Exécute fclean puis relance le DEV"

# ─── MODE DÉVELOPPEMENT (Hot-Reload) ──────────────────────────────────────────

dev:
	@mkdir -p $(PATH_VOLUME)
	$(COMPOSE_DEV) up -d
	@echo "$(GREEN)🚀 Environnement DEV lancé ! (DB stockée dans $(PATH_VOLUME))$(RESET)"

logs:
	$(COMPOSE_DEV) logs -f

logs-%:
	$(COMPOSE_DEV) logs -f $*

restart:
	$(COMPOSE_DEV) restart

# ─── MODE PRODUCTION (binaire Rust, image runtime légère) ─────────────────────

prod:
	@mkdir -p $(PATH_VOLUME)
	$(COMPOSE_PROD) up -d --build
	@echo "$(GREEN)Environnement PROD lancé !$(RESET)"

prod-logs:
	$(COMPOSE_PROD) logs -f

prod-logs-%:
	$(COMPOSE_PROD) logs -f $*

backup-prod:
	@mkdir -p $(BACKUP_DIR)
	$(COMPOSE_PROD) exec -T db pg_dump -U $(DB_USER) --clean --if-exists $(DB_NAME) \
		> $(BACKUP_DIR)/backup_$$(date +%F_%H%M).sql
	@echo "Backup créé dans $(BACKUP_DIR)/"

restore-prod:
	@test -n "$(FILE)" || (echo "Usage: make restore-prod FILE=backups/xxx.sql" && exit 1)
	$(COMPOSE_PROD) exec -T db psql -U $(DB_USER) $(DB_NAME) < $(FILE)

# ─── ARRÊT ET NETTOYAGE ───────────────────────────────────────────────────────

# Arrête les conteneurs sans rien supprimer
down:
	$(COMPOSE_ALL) stop
	@echo "$(YELLOW)Conteneurs arrêtés (conteneurs, images et données conservés).$(RESET)"

# Supprime conteneurs et images du projet (garde les volumes et la DB)
clean:
	$(COMPOSE_ALL) down --rmi all
	@echo "$(RED)Conteneurs et images supprimés (volumes et DB conservés).$(RESET)"

# Supprime images, volumes Docker, dossier physique de la DB et prune le système
fclean:
	$(COMPOSE_ALL) down -v --rmi all
	@if [ -d "$(PATH_VOLUME)" ]; then \
		$(DOCKER) run --rm -v "$(CURDIR):/work" alpine rm -rf "/work/$(PATH_VOLUME)"; \
	fi
	$(DOCKER) system prune -af --volumes
	@echo "$(RED)Nettoyage complet terminé (images, volumes, données DB et cache Docker).$(RESET)"

re: fclean dev

# ─── Logs & Debug ─────────────────────────────────────────────────────────────

ps:
	$(DOCKER) ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

in-%:
	$(COMPOSE_DEV) exec $* sh

# ─── Tests ────────────────────────────────────────────────────────────────────


unit:
	cargo test --lib

test-all:
	cargo test

unit-docker:
	$(DOCKER) run --rm \
		-v $(CURDIR):/app \
		-v cargo-registry:/usr/local/cargo/registry \
		-v cargo-target:/app/target \
		-w /app \
		$(RUST_IMAGE) \
		cargo test --lib

rename:
	sed -i '0,/^name = ".*"/s//name = "$(PROJECT_NAME)"/' Cargo.toml
