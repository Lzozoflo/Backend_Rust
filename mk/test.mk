# ─── Tests & qualité ──────────────────────────────────────────────────────────

RUST_IMAGE ?= rust:1-slim-bookworm

.PHONY: test-db lint fmt

# Parcours complet user contre une vraie base Postgres (tests/db_flow.rs)
test-db:
	@mkdir -p $(PATH_DATABASE_VOLUME)
	$(COMPOSE_DEV) run --rm backend cargo test --test db_flow -- --ignored

lint:
	$(DOCKER) run --rm -v $(CURDIR):/app -v cargo-registry:/usr/local/cargo/registry -w /app $(RUST_IMAGE) \
		sh -c "rustup component add clippy rustfmt && cargo fmt --check && cargo clippy --all-targets -- -D warnings"

fmt:
	$(DOCKER) run --rm -v $(CURDIR):/app -w /app $(RUST_IMAGE) sh -c "rustup component add rustfmt && cargo fmt"
