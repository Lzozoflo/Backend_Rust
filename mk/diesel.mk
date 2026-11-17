# ─── Diesel : migrations générées automatiquement ────────────────────────────
# Le CLI `diesel` est installé dans l'image dev (Dockerfile, étape `dev`).

DIESEL = $(COMPOSE_DEV) run --rm -T backend diesel

# 1. Tu modifies src/schema.rs
# 2. make migration name=add_role   → génère migrations/<date>_add_role/{up,down}.sql
# 3. make migrate                   → applique + réécrit src/schema.rs
migration:
	@test -n "$(name)" || { echo "$(RED)usage : make migration name=ajout_colonne$(RESET)"; exit 1; }
	$(DIESEL) migration generate --diff-schema $(name)
	@echo "$(GREEN)✅ Migration générée : relis up.sql / down.sql puis make migrate$(RESET)"

# Migration vide à remplir à la main (index fonctionnels, données, triggers…)
migration-empty:
	@test -n "$(name)" || { echo "$(RED)usage : make migration-empty name=xxx$(RESET)"; exit 1; }
	$(DIESEL) migration generate $(name)

migrate:
	$(DIESEL) migration run

revert:
	$(DIESEL) migration revert

redo:
	$(DIESEL) migration redo

# Réécrit src/schema.rs à partir de la base
schema:
	$(DIESEL) print-schema > src/schema.rs
