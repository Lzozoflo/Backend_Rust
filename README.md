# Backend_Rust — Axum + Diesel

Backend HTTP en Rust : inscription, connexion et gestion de compte.
Stack : **Axum** (HTTP) · **Diesel + diesel-async** (PostgreSQL, requêtes paramétrées, migrations générées) · **JWT** · **Argon2id**.

---


## Démarrage rapide

```bash
cp .env.example .env     # optionnel : ports, secret JWT…
make dev                 # Postgres + backend (hot-reload) + Adminer
curl localhost:8081/health
```

| Service  | URL                     |
|----------|-------------------------|
| API      | http://localhost:8081   |
| Adminer  | http://localhost:5051   |

Adminer : système `PostgreSQL`, serveur `db`, utilisateur `postgres`, mot de passe `postgres`, base `tsg_db`.

Prérequis : Docker + Docker Compose, `make`. Rust n'est nécessaire que pour `make unit-local` / `make test-all`.

---

## Structure

Les préfixes `f1_` à `f4_` (flow) gardent les dossiers dans l'ordre du flux d'une requête.
Une entité = un fichier au nom de l'entité dans chaque couche. Les `mod.rs` ne font que déclarer des modules.

```
src/
├─ f1_route/            routes HTTP
│  ├─ mod.rs              router (routes, CORS, trace, fallback 404)
│  ├─ health.rs           GET /health
│  ├─ user.rs             POST /register, POST /login, PUT /user, DELETE /user
│  └─ extract.rs          ValidJson<T> (JSON + validation) et AuthUser (Bearer JWT)
├─ f2_dto/              formes des données entrantes / sortantes
│  ├─ user_in.rs          RegisterRequest, LoginRequest, UpdateUserRequest + règles de validation
│  ├─ user_out.rs         UserResponse, AuthResponse
│  ├─ health_out.rs       HealthResponse
│  └─ validate.rs         trait Validate
├─ f3_services/         logique métier
│  └─ user/
│     ├─ user.rs          register, login, update, delete
│     ├─ password.rs      hachage / vérification Argon2id
│     └─ token.rs         création / vérification des JWT
├─ f4_repo/             accès base de données (SQL)
│  ├─ db.rs               pool bb8 + migrations embarquées
│  ├─ user.rs             requêtes SQL de l'entité user
│  └─ error.rs            diesel::result::Error → AppError
├─ schema.rs            schéma Diesel (source de vérité des migrations)
├─ config.rs            variables d'environnement
├─ error.rs             AppError (réponses JSON d'erreur)
├─ lib.rs / main.rs
migrations/
└─ 2026-11-13-100000_create_users/{up,down}.sql
diesel.toml             config du CLI Diesel
build.rs                recompile quand une migration est ajoutée
tests/
├─ api.rs               intégration HTTP (sans base)
├─ db_flow.rs           parcours complet contre Postgres (ignoré par défaut)
└─ common/mod.rs        helpers
```

### Flux d'une requête

```
f1_route  →  f2_dto (validate)  →  f3_services  →  f4_repo  →  PostgreSQL
```

- L'extracteur `ValidJson` désérialise le corps **puis appelle `validate()`** du DTO : si le handler s'exécute, l'entrée est valide. Une entrée invalide n'atteint jamais les services.
- Les services ne contiennent aucun SQL ; `f4_repo` ne contient aucune logique métier.

---

## Migrations (générées automatiquement)

`src/schema.rs` est la source de vérité. Tu le modifies, Diesel écrit le SQL :

```bash
# 1. édite src/schema.rs (ajoute une colonne / une table dans diesel::table! { ... })
make migration name=add_role   # 2. génère migrations/<date>_add_role/{up,down}.sql
make migrate                   # 3. applique + réécrit src/schema.rs depuis la base
make revert                    #    annule la dernière migration
make migration-empty name=xxx  #    migration vide, SQL à la main (index fonctionnels, données…)
```

- Les migrations sont aussi appliquées au démarrage du serveur (`f4_repo::migrate`).
- Relis toujours `up.sql` : un renommage de colonne est vu comme « drop + add » par `--diff-schema`.
- Après avoir ajouté la table dans `schema.rs`, ajoute la struct `Queryable` correspondante dans `f4_repo/`.

---

## API

| Méthode | Route       | Auth   | Corps JSON                          | Succès               |
|---------|-------------|--------|-------------------------------------|----------------------|
| GET     | `/health`   | non    | —                                   | `200`                |
| POST    | `/register` | non    | `username`, `email`, `password`     | `201` + utilisateur  |
| POST    | `/login`    | non    | `email`, `password`                 | `200` + token        |
| PUT     | `/user`     | Bearer | `username?`, `email?`, `password?`  | `200` + utilisateur  |
| DELETE  | `/user`     | Bearer | —                                   | `204`                |

```bash
# Inscription
curl -X POST localhost:8081/register -H 'content-type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"motdepasse1"}'

# Connexion → récupère le token
curl -X POST localhost:8081/login -H 'content-type: application/json' \
  -d '{"email":"alice@example.com","password":"motdepasse1"}'

# Modification (seuls les champs fournis changent)
curl -X PUT localhost:8081/user -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{"username":"alice_v2"}'

# Suppression
curl -X DELETE localhost:8081/user -H "authorization: Bearer $TOKEN"
```

### Réponses

```jsonc
// POST /register, PUT /user
{ "id": "…", "username": "alice", "email": "alice@example.com", "created_at": "…" }

// POST /login
{ "token": "…", "token_type": "Bearer", "expires_in": 3600, "user": { … } }

// Toute erreur
{ "error": "message lisible" }
```

| Code | Signification                                                        |
|------|----------------------------------------------------------------------|
| 400  | Entrée invalide (validation du DTO ou JSON mal formé)                |
| 401  | Identifiants invalides, token absent / invalide / expiré             |
| 404  | Utilisateur ou route introuvable                                     |
| 409  | `email déjà utilisé` ou `username déjà utilisé`                      |
| 500  | Erreur interne (le détail est loggé, jamais renvoyé au client)       |

### Validation : elle se fait dans le DTO (`f2_dto/user_in.rs`)

**Règle du projet : c'est le DTO d'entrée qui valide son contenu**, pas la route ni le service.
Chaque DTO d'entrée implémente le trait `Validate` (`f2_dto/validate.rs`) :

```rust
// f2_dto/user_in.rs
impl Validate for RegisterRequest {
    fn validate(&self) -> Result<(), AppError> {
        check_username(&self.username)?;
        check_email(&self.email)?;
        check_password(&self.password)
    }
}
```

Le handler n'appelle rien à la main : il déclare simplement `ValidJson<RegisterRequest>` en paramètre.
L'extracteur (`f1_route/extract.rs`) désérialise le JSON puis appelle `validate()` **avant** que le handler ne s'exécute :

```rust
pub async fn register(
    State(state): State<AppState>,
    ValidJson(req): ValidJson<RegisterRequest>,   // ← déjà validé ici
) -> Result<…> { … }
```

Si `validate()` renvoie une erreur, la requête est rejetée en `400` et ni le service ni la base ne sont appelés.

**Pour ajouter ou modifier une règle** : édite le `check_*` concerné (ou la méthode `validate()`) dans `f2_dto/user_in.rs`, et ajoute le test correspondant dans le bloc `UNIT TEST` du même fichier.
**Pour un nouveau DTO d'entrée** : crée la struct, implémente `Validate`, puis utilise `ValidJson<MonDto>` dans la route.

> Les règles actuellement codées ne sont que des **exemples** à adapter à ton jeu :
> - **username** : 3 à 32 caractères, lettres / chiffres / `_` / `-`
> - **email** : format valide, 254 caractères max (stocké en minuscules)
> - **password** : 8 caractères min, 128 octets max, au moins une lettre et un chiffre
> - **update** : au moins un champ à modifier
>
> Certaines garanties vivent aussi en base, en plus du DTO : l'unicité de l'`email` et du `username` (sans tenir compte de la casse) est assurée par la migration `0001_create_users.sql`, ce qui donne les `409` à l'inscription.

---

## Configuration

| Variable           | Défaut (dev)                    | Rôle                                   |
|--------------------|---------------------------------|----------------------------------------|
| `DATABASE_URL`     | `postgres://…@localhost/tsg_db` | Connexion PostgreSQL                   |
| `PORT`             | `8080`                          | Port d'écoute dans le conteneur        |
| `APP_VERSION`      | `0.1`                           | Renvoyée par `/health`                 |
| `TOKEN_SECRET`     | secret de dev                   | Clé de signature des JWT               |
| `TOKEN_EXPIRATION` | `3600`                          | Durée de vie d'un token (secondes)     |
| `RUST_LOG`         | `info,tower_http=debug`         | Niveau de logs                         |

Côté Docker (voir `.env.example`) : `HOST_PORT` (8081), `ADMINER_PORT` (5051).

- Une variable présente mais invalide fait **échouer le démarrage** (fail fast).
- En **prod**, `TOKEN_SECRET` est obligatoire (`make prod` refuse de démarrer sans). En dev, le secret par défaut déclenche un avertissement au lancement.

---

## Commandes `make`

`make help` affiche la liste. Les principales :

| Commande            | Effet                                                          |
|---------------------|----------------------------------------------------------------|
| `make dev`          | Environnement de dev (hot-reload via `cargo-watch`) + Adminer  |
| `make prod`         | Build de l'image de production et démarrage                    |
| `make logs`         | Suit les logs (`logs-backend`, `logs-db`)                      |
| `make ps`           | État des conteneurs                                            |
| `make down`         | Arrête les conteneurs (garde la base)                          |
| `make down-v`       | Arrête et supprime les volumes Docker                          |
| `make db-export`    | Sauvegarde la base dans `~/backup_tsg_sqlx_db.sql`             |
| `make db-import`    | Restaure la base depuis cette sauvegarde                       |
| `make db-shell`     | Ouvre `psql` dans la base                                      |
| `make in-backend`   | Shell dans le conteneur backend                                |
| `make re`           | `fclean` puis relance le dev (**destructif**)                  |

Les données PostgreSQL sont stockées sur l'hôte dans `~/tsg_sqlx_db_data` (modifiable avec `PATH_DATABASE_VOLUME`).

> ⚠️ `make prune`, `rmi`, `volumes`, `clean`, `fclean` et `re` agissent sur **tout** Docker de la machine, pas seulement ce projet.

---

## Tests

| Commande           | Contenu                                                              |
|--------------------|----------------------------------------------------------------------|
| `make unit`        | Tests unitaires dans Docker (`cargo test --lib`)                     |
| `make unit-local`  | Idem avec le Cargo installé sur ta machine (plus rapide)             |
| `make test-all`    | Unitaires + intégration HTTP sans base                               |
| `make test-db`     | Parcours complet (register → login → update → delete) sur Postgres   |
| `make lint`        | `cargo fmt --check` + `cargo clippy -D warnings` (dans Docker)       |
| `make fmt`         | Formate le code                                                      |

Les tests unitaires sont **dans les fichiers sources**, après le bandeau :

```rust
// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests { … }
```

`tests/db_flow.rs` est marqué `#[ignore]` : il ne tourne qu'avec `make test-db`.

---

## Base de données

- Une migration = un fichier `migrations/NNNN_nom.sql`, **embarqué dans le binaire** (`sqlx::migrate!`) et appliqué automatiquement au démarrage.
- Pour modifier le schéma : ajoute `0002_….sql`. Ne modifie jamais une migration déjà appliquée.
- Garde-fous dans le schéma : `email` unique et forcé en minuscules, `username` unique sans casse.

### Règle SQLx anti-injection

Toute valeur venant de l'extérieur passe par `.bind()` (paramètres `$1, $2…`) :

```rust
sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1").bind(email)
```

**Jamais** de `format!` ni de concaténation pour construire une requête avec une donnée utilisateur.

---

## Sécurité — résumé

- Mots de passe hachés en **Argon2id** avec sel aléatoire ; hachage exécuté hors du runtime async (`spawn_blocking`).
- Login : même réponse (`401 identifiants invalides`) pour un email inconnu ou un mauvais mot de passe, avec un coût de calcul équivalent pour ne pas révéler l'existence d'un compte.
- JWT HS256 signé avec `TOKEN_SECRET`, expiration configurable.
- Les réponses ne contiennent jamais le hash du mot de passe ; `Config` n'affiche jamais le secret dans les logs.
- CORS **permissif** (toutes origines) : à restreindre avant une vraie mise en production.
- Conteneur de production exécuté en **utilisateur non-root**.

---

## Dépannage

| Symptôme                                         | Piste                                                                 |
|--------------------------------------------------|-----------------------------------------------------------------------|
| `make prod` : « TOKEN_SECRET est obligatoire »   | Définis `TOKEN_SECRET` dans `.env`                                    |
| Le backend ne se connecte pas à la base          | `make logs-db` ; attendre le healthcheck de `db`                      |
| Port déjà utilisé                                | Change `HOST_PORT` / `ADMINER_PORT` dans `.env`                       |
| `make dev` échoue sur le volume                  | Le dossier `~/tsg_sqlx_db_data` doit exister (`make dev` le crée)     |
| Première compilation très longue                 | Normal : les dépendances sont mises en cache (volumes Docker)         |

---

## À venir

- CI GitHub Actions (build + `cargo test`), une fois les tests validés en local.
