# ─── Tests & qualité ──────────────────────────────────────────────────────────

RUST_IMAGE ?= rust:1-slim-bookworm
CURL       ?= curl -sS -w "\nHTTP %{http_code}\n"
API_URL    ?= http://127.0.0.1:$(or $(HOST_PORT),8081)
BLUE       ?= $(CYAN)

ECHO = @echo "\n$(BLUE)------------------------------------------------------------------$(RESET)\n$(YELLOW)[➡] Executing:$(RESET)"

TEST_USER  ?= client_testee
TEST_EMAIL ?= teste@exemple.com
TEST_PASS  ?= MonMotDePasse123!

ADMIN_USER  ?= admin
ADMIN_EMAIL ?= admin@exemple.com
ADMIN_PASS  ?= AdminPassword123!

USER  ?= $(ADMIN_USER)
EMAIL ?= $(ADMIN_EMAIL)
PASS  ?= $(ADMIN_PASS)

.PHONY: test-db lint fmt all-test health user-service register login update-user delete

# ─── Tests Cargo / Rust ───────────────────────────────────────────────────────

# Parcours complet user contre une vraie base Postgres (tests/db_flow.rs)
test-db:
	@mkdir -p $(PATH_DATABASE_VOLUME)
	$(COMPOSE_DEV) run --rm backend cargo test --test db_flow -- --ignored

lint:
	$(DOCKER) run --rm -v $(CURDIR):/app -v cargo-registry:/usr/local/cargo/registry -w /app $(RUST_IMAGE) \
		sh -c "rustup component add clippy rustfmt && cargo fmt --check && cargo clippy --all-targets -- -D warnings"

fmt:
	$(DOCKER) run --rm -v $(CURDIR):/app -w /app $(RUST_IMAGE) sh -c "rustup component add rustfmt && cargo fmt"

# ─── Tests HTTP (curl) ────────────────────────────────────────────────────────

all-test:
	@$(MAKE) health
	@$(MAKE) user-service

health:
	$(ECHO) GET $(API_URL)/health
	@$(CURL) -X GET $(API_URL)/health

# ─── ORCHESTRATION DES SERVICES ───────────────────────────────────────────────

user-service:
	@$(MAKE) register
	@$(MAKE) login
	@$(MAKE) update-user
	@$(MAKE) delete

# ─── COMMANDES INDIVIDUELLES USER ─────────────────────────────────────────────

register:
	$(ECHO) POST $(API_URL)/register
	@$(CURL) -X POST $(API_URL)/register \
		-H "Content-Type: application/json" \
		-d '{"username": "$(USER)", "email": "$(EMAIL)", "password": "$(PASS)"}'

login:
	$(ECHO) POST $(API_URL)/login
	@$(CURL) -X POST $(API_URL)/login \
		-H "Content-Type: application/json" \
		-d '{"email": "$(EMAIL)", "password": "$(PASS)"}' > login_resp.txt
	@cat login_resp.txt
	@grep -o '"token":[[:space:]]*"[^"]*"' login_resp.txt | head -n 1 | sed 's/"token":[[:space:]]*"//;s/"//' > token.txt
	@rm -f login_resp.txt

update-user:
	$(ECHO) PUT $(API_URL)/user
	@TOKEN=$$(cat token.txt 2>/dev/null); \
	$(CURL) -X PUT $(API_URL)/user \
		-H "Content-Type: application/json" \
		-H "Authorization: Bearer $$TOKEN" \
		-d '{"username": "$(USER)_v2"}'

delete:
	$(ECHO) DELETE $(API_URL)/user
	@TOKEN=$$(cat token.txt 2>/dev/null); \
	$(CURL) -X DELETE $(API_URL)/user \
		-H "Authorization: Bearer $$TOKEN"
	@rm -f token.txt
