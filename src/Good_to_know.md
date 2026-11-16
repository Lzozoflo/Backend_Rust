# Architecture Technique — Backend Rust (`Axum` + `Diesel`)

Ce document détaille la conception logicielle, le flux des données, l'organisation des couches et l'infrastructure de déploiement du backend.

---

## 1. Vue d'ensemble et Philosophie

Le projet repose sur une **architecture en couches orientée flux (Flow Architecture)** conçue pour garantir une séparation stricte des responsabilités, une maintenabilité élevée et une sécurité par défaut (validation précoce, typage fort, requêtes paramétrées).

### Stack Technique
* **Serveur HTTP & Routage** : `Axum` (v0.8) propulsé par le runtime asynchrone `Tokio`.
* **Middlewares** : `Tower` & `Tower-HTTP` (CORS, traçage des requêtes).
* **ORM & Base de données** : `PostgreSQL 16`, `Diesel` (v2.3) + `diesel-async` avec pool de connexions `bb8`.
* **Sécurité & Authentification** : `Argon2id` (hachage de mots de passe) et `JWT` (`jsonwebtoken` HS256).
* **Gestion des erreurs** : `thiserror` avec conversion centralisée vers des réponses HTTP JSON standardisées.

---

## 2. Organisation du Code (`src/`)

Les dossiers principaux sont préfixés de **`f1_` à `f4_` (Flow)** afin que l'arborescence de l'IDE reflète naturellement l'ordre chronologique de traitement d'une requête HTTP.

> **Convention de nommage :** Une entité métier = un fichier portant son nom dans chaque couche (ex: `user.rs`). Les fichiers `mod.rs` sont strictement réservés à la déclaration et à l'exportation des modules.

```text
src/
├── f1_route/              # [Couche 1] Exposition HTTP & Extraction
│   ├── mod.rs             # Configuration du Router Axum, CORS, Tracing, Fallback 404
│   ├── health.rs          # Endpoint de santé (GET /health)
│   ├── user.rs            # Endpoints utilisateurs (/register, /login, /user)
│   └── extract.rs         # Extracteurs personnalisés : ValidJson<T> et AuthUser
│
├── f2_dto/                # [Couche 2] Contrats d'interface & Validation
│   ├── validate.rs        # Trait générique Validate
│   ├── user_in.rs         # DTOs entrants (Register, Login, Update) + règles métier de format
│   ├── user_out.rs        # DTOs sortants (UserResponse, AuthResponse)
│   └── health_out.rs      # DTO sortant pour le healthcheck
│
├── f3_services/           # [Couche 3] Logique Métier & Sécurité
│   └── user/
│       ├── user.rs        # Orchestration des cas d'usage (inscription, connexion, maj, suppression)
│       ├── password.rs    # Hachage et vérification Argon2id (déportés sur spawn_blocking)
│       └── token.rs       # Génération et validation des jetons JWT
│
├── f4_repo/               # [Couche 4] Persistance & Accès aux Données
│   ├── db.rs              # Initialisation du pool bb8 et exécution des migrations embarquées
│   ├── user.rs            # Requêtes SQL typées (Diesel) pour l'entité User
│   └── error.rs           # Mapping des erreurs Diesel (ex: UniqueViolation -> 409 Conflict)
│
├── schema.rs              # Source de vérité du schéma SQL (utilisé par Diesel pour le diff)
├── config.rs              # Chargement et validation "Fail-Fast" des variables d'environnement
├── error.rs               # Enumération centrale AppError (IntoResponse)
├── lib.rs                 # Définition de l'état global (AppState) et exposition des modules
└── main.rs                # Point d'entrée : init logs, config, migrations, pool DB et serveur TCP
```

---

## 3. Cycle de Vie d'une Requête

Le flux est strictement unidirectionnel. Une couche supérieure ne peut appeler que la couche immédiatement inférieure :

```text
Client HTTP
    │
    ▼
┌──────────────────────────────────────────────────────────┐
│ f1_route (Axum Router + Extracteurs)                     │
│  ├─ AuthUser : vérifie le header Authorization: Bearer   │
│  └─ ValidJson<T> : désérialise le JSON                   │
└─────────────────────────┬────────────────────────────────┘
                          │ Appelle validate()
                          ▼
┌──────────────────────────────────────────────────────────┐
│ f2_dto (Data Transfer Objects)                           │
│  └─ Vérifie les contraintes (longueur, regex, format)    │
│     [Rejet immédiat en 400 Bad Request si invalide]      │
└─────────────────────────┬────────────────────────────────┘
                          │ Passe un DTO garanti valide
                          ▼
┌──────────────────────────────────────────────────────────┐
│ f3_services (Logique Métier)                             │
│  ├─ Applique les règles métier                           │
│  ├─ Hache les mots de passe (Argon2id) / Signe les JWT   │
│  └─ Ne contient AUCUNE requête SQL                       │
└─────────────────────────┬────────────────────────────────┘
                          │ Appelle le repository
                          ▼
┌──────────────────────────────────────────────────────────┐
│ f4_repo (Accès Base de Données)                          │
│  ├─ Exécute les requêtes Diesel sur le pool async bb8    │
│  └─ Ne contient AUCUNE logique métier                    │
└─────────────────────────┬────────────────────────────────┘
                          │
                          ▼
                     PostgreSQL 16
```

### Principes Clés du Flux
1. **Validation à la frontière (`ValidJson`)** : Les handlers dans `f1_route` ne manipulent jamais de données brutes non vérifiées. Si `validate()` échoue, les couches `f3_services` et `f4_repo` ne sont jamais sollicitées.
2. **Gestion centralisée des erreurs (`AppError`)** : Toutes les couches retournent `Result<T, AppError>`. L'implémentation de `IntoResponse` sur `AppError` traduit automatiquement chaque erreur en code HTTP approprié (`400`, `401`, `404`, `409`, `500`) avec un corps JSON uniforme `{"error": "..."}`, tout en masquant les détails internes des erreurs `500`.
3. **Isolation du CPU-bound** : Le hachage `Argon2id` étant coûteux en CPU, il est isolé via `tokio::task::spawn_blocking` dans `f3_services/user/password.rs` pour ne jamais bloquer les threads d'I/O d'Axum.

---

## 4. Gestion de la Base de Données & Migrations

L'architecture adopte une approche **Schema-First** assistée par le CLI Diesel :

* **Source de vérité** : Le fichier `src/schema.rs` définit la structure des tables via la macro `diesel::table!`.
* **Génération automatique** : La commande `make migration name=...` utilise `diesel migration generate --diff-schema` pour comparer `src/schema.rs` à l'état actuel de la base et générer automatiquement les fichiers `up.sql` et `down.sql`.
* **Embarquement à la compilation** : Grâce à `diesel_migrations::embed_migrations!` et au script `build.rs` (qui surveille le dossier `migrations/`), toutes les migrations sont compilées directement dans le binaire final et exécutées automatiquement au démarrage de l'application (`f4_repo::migrate`).

---

## 5. Infrastructure, Docker & Tooling

L'environnement est entièrement conteneurisé et orchestré via un `Makefile` modulaire (`mk/env.mk`, `mk/diesel.mk`, `mk/test.mk`).

### Architecture Multi-Stage (`Dockerfile`)
Le `Dockerfile` définit trois cibles distinctes :
1. **`dev`** : Image complète incluant `cargo-watch` (hot-reload sur modification de `src/`, `migrations/`, `Cargo.toml`) et `diesel_cli`.
2. **`builder`** : Compile le binaire en mode `--release` en utilisant un système de mise en cache des dépendances (compilation préalable d'un `main.rs` factice).
3. **`runtime`** : Image finale ultra-légère (`debian:bookworm-slim`) contenant uniquement le binaire compilé, exécuté sous un utilisateur système non-privilégié (`tsg:tsg`) pour des raisons de sécurité.

### Topologie Docker Compose
* `docker-compose.yml` : Définit le socle commun (service `db` PostgreSQL 16 avec `healthcheck` et persistance locale via volume bind).
* `docker-compose.dev.yml` : Ajoute le backend en mode hot-reload (volumes montés sur le code et caches Cargo) ainsi qu'**Adminer** (port `5051`) pour l'inspection visuelle de la base.
* `docker-compose.prod.yml` : Déploie uniquement la base de données et le conteneur `runtime` avec redémarrage automatique et vérification stricte de `TOKEN_SECRET`.

---

## 6. Stratégie de Tests

La pyramide de tests est découpée en trois niveaux isolés :

1. **Tests Unitaires (`cargo test --lib`)** :
   * Colocalisés directement au bas des fichiers sources concernés (sous le bandeau `// ═══ UNIT TEST ═══`).
   * Testent les règles de validation des DTOs, le hachage Argon2 et la signature JWT sans aucune dépendance externe.
2. **Tests d'Intégration HTTP sans BDD (`tests/api.rs`)** :
   * Utilisent `tower::ServiceExt::oneshot` et un pool de connexion paresseux (`connect_lazy`).
   * Vérifient le routage, les en-têtes CORS, le rejet des JSON malformés, la validation des DTOs et le contrôle d'accès JWT (`401 Unauthorized`) sans nécessiter de base PostgreSQL active.
3. **Tests End-to-End avec BDD (`tests/db_flow.rs`)** :
   * Marqués `#[ignore]` par défaut et exécutés via `make test-db`.
   * Jouent le cycle de vie complet d'un utilisateur (inscription, gestion des conflits d'unicité insensibles à la casse, connexion, mise à jour, suppression) contre une véritable instance PostgreSQL.

---

## 7. Guide d'Extension (Ajouter une Nouvelle Entité)

Pour ajouter une nouvelle ressource (ex: `item`), suivre l'ordre naturel de l'architecture :

1. **Base de données (`schema.rs` & `migrations/`)** :
   * Ajouter la table dans `src/schema.rs`.
   * Générer la migration avec `make migration name=create_items`, vérifier le SQL généré, puis appliquer avec `make migrate`.
2. **Repository (`src/f4_repo/item.rs`)** :
   * Créer les structs `Queryable` / `Insertable` et implémenter les requêtes Diesel.
3. **DTOs (`src/f2_dto/item_in.rs` & `item_out.rs`)** :
   * Définir les payloads JSON et implémenter le trait `Validate` sur les structures entrantes.
4. **Service (`src/f3_services/item/item.rs`)** :
   * Implémenter la logique métier en appelant `f4_repo::item`.
5. **Routes (`src/f1_route/item.rs`)** :
   * Créer les handlers Axum utilisant `ValidJson<T>` et `AuthUser`, puis enregistrer les routes dans `src/f1_route/mod.rs`.