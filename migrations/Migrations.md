# Migrations de base de données : guide à suivre à chaque changement

Ce guide explique comment modifier la base **sans perdre de données**.
Il est écrit pour quelqu'un qui n'est pas à l'aise en SQL.

## Les 3 idées à retenir

1. **Le volume garde les données, la migration fait évoluer la structure.** Les deux sont indépendants.
2. **Une migration, c'est un fichier SQL** dans `migrations/<date>_<nom>/` (`up.sql` pour appliquer, `down.sql` pour annuler). Postgres retient lesquelles sont déjà appliquées (table `__diesel_schema_migrations`) et ne rejoue que les nouvelles.
3. **Diesel génère le SQL depuis `schema.rs`, mais il ne connaît pas tes données.** Il peut donc produire un SQL qui plante ou qui supprime des données. C'est à toi (ou à une IA) de le relire **avant** de l'appliquer.

## Le workflow complet

### Étape 1 : en dev

| # | Action | Pourquoi |
|---|--------|----------|
| 1 | Modifier `src/schema.rs` | Tu décris la structure voulue |
| 2 | `make migration name=ajout_xxx` | Diesel écrit `up.sql` et `down.sql` |
| 3 | **Relire `up.sql` et `down.sql`** (voir la checklist plus bas) | C'est LA vérification. Rien n'est encore appliqué |
| 4 | Corriger le SQL si besoin | Voir « Les cas à corriger » |
| 5 | `make migrate` | Applique la migration et réécrit `schema.rs` |
| 6 | `make redo` | Annule puis rejoue : prouve que `down.sql` marche |
| 7 | `make test-db` | Parcours complet contre une vraie base |

> ⚠️ Si tu modifies `schema.rs` puis lances `make migrate` **sans** avoir fait `make migration`, ta modification est écrasée (`migrate` réécrit le fichier depuis la base).

### Étape 2 : tester sur une copie des vraies données de prod

C'est l'étape qui évite le plus de catastrophes : une migration peut marcher sur ta base de dev (vide ou presque) et planter sur la prod (pleine).

```bash
# 1. Récupérer un dump de la prod (sur le serveur)
make backup-prod

# 2. Remettre la base de dev à zéro, puis y restaurer le dump
DEVC="docker compose -f docker-compose.yml -f docker-compose.dev.yml"
$DEVC exec -T db psql -U postgres <PROJECT_NAME>_db -c 'DROP SCHEMA public CASCADE; CREATE SCHEMA public;'
$DEVC exec -T db psql -U postgres <PROJECT_NAME>_db < backups/backup_XXXX.sql

# 3. Appliquer la nouvelle migration sur ces données réelles
make migrate
```

Remplace `<PROJECT_NAME>` par la valeur de ton `.env`. Si `make migrate` passe ici, il passera en prod.
Le reset du schéma (`DROP SCHEMA ...`) est nécessaire : sans lui, les tables créées par ta nouvelle migration existeraient déjà et le test serait faussé.

### Étape 3 : déployer

1. Commit et push : le dossier `migrations/` **et** `src/schema.rs`.
2. Sur le serveur : `make deploy-prod` (fait un backup, rebuild, relance).
3. Le backend applique les migrations au démarrage. Vérifie : `make prod-logs`.
4. Teste `/health`, puis un login.

### Si ça casse en prod

```bash
docker compose -f docker-compose.yml -f docker-compose.prod.yml stop backend   # arrêter le backend d'abord !
make restore-prod FILE=backups/backup_XXXX.sql                                  # restaurer le dernier backup
```

Ensuite, remets le code d'avant (`git revert`) avant de relancer, sinon le backend rejouera la migration qui a échoué.

## Checklist : que chercher dans `up.sql`

Ouvre le `up.sql` généré et cherche ces mots-clés :

| Tu vois | Risque | Que faire |
|---------|--------|-----------|
| `DROP TABLE` | Table et données supprimées | Voulu ? Sinon, ne pas appliquer |
| `DROP COLUMN` | Colonne et données supprimées | Voulu ? Sinon, ne pas appliquer |
| `DROP COLUMN` **et** `ADD COLUMN` pour un nom similaire | Un **renommage** mal détecté : la donnée est perdue | Voir « Renommer » |
| `ADD COLUMN ... NOT NULL` sans `DEFAULT` | **Plante** si la table contient des lignes | Ajouter un `DEFAULT` |
| `ALTER COLUMN ... TYPE` | Peut échouer ou convertir mal | Ajouter `USING`, tester sur la copie |
| `SET NOT NULL` | Plante s'il existe des valeurs `NULL` | Remplir les `NULL` avant |
| `CREATE UNIQUE INDEX` / `UNIQUE` | Plante s'il y a déjà des doublons | Vérifier les doublons d'abord |
| `ADD ... FOREIGN KEY` / `REFERENCES` | Plante si des lignes pointent vers rien | Vérifier les données |
| `TRUNCATE` / `DELETE` | Données supprimées | Ne devrait jamais apparaître sans que tu l'aies voulu |

Relis aussi `down.sql` : c'est l'inverse de `up.sql`. Un `down.sql` qui supprime une colonne détruit ses données, donc **le backup reste ton vrai filet de sécurité**, pas `make revert`.

## Les cas à corriger (modèles de SQL)

### Ajouter une colonne `NOT NULL` à une table qui a déjà des lignes

```sql
-- ❌ plante en prod
ALTER TABLE "users" ADD COLUMN "token_version" INT4 NOT NULL;

-- ✅ avec une valeur par défaut pour les lignes existantes
ALTER TABLE "users" ADD COLUMN "token_version" INT4 NOT NULL DEFAULT 0;
```

### Ajouter une colonne `NOT NULL` dont la valeur doit être calculée

```sql
ALTER TABLE "users" ADD COLUMN "display_name" TEXT;              -- 1. colonne nullable
UPDATE "users" SET "display_name" = "username";                  -- 2. remplir les lignes existantes
ALTER TABLE "users" ALTER COLUMN "display_name" SET NOT NULL;    -- 3. imposer NOT NULL
```

### Renommer une colonne ou une table

Si tu renommes dans `schema.rs`, Diesel génère `DROP` + `ADD` (donnée perdue). À la place :

1. Annule le changement de `schema.rs` (ou supprime le dossier de migration généré, s'il n'est pas encore appliqué).
2. `make migration-empty name=rename_xxx`
3. Écris à la main :

```sql
-- up.sql
ALTER TABLE "users" RENAME COLUMN "ancien_nom" TO "nouveau_nom";
-- down.sql
ALTER TABLE "users" RENAME COLUMN "nouveau_nom" TO "ancien_nom";
```

4. `make migrate` (Diesel mettra `schema.rs` à jour).

### Changer le type d'une colonne

```sql
ALTER TABLE "tickets" ALTER COLUMN "price" TYPE BIGINT USING "price"::BIGINT;
```

### Ajouter une contrainte `UNIQUE` sur une table qui a des données

Vérifie d'abord qu'il n'y a pas de doublons :

```sql
SELECT "email", COUNT(*) FROM "users" GROUP BY "email" HAVING COUNT(*) > 1;
```

Si cette requête renvoie des lignes, nettoie les données avant d'ajouter la contrainte.

### Supprimer une colonne ou une table (volontairement)

- Fais un `make backup-prod` juste avant.
- Pour les cas sensibles : d'abord un déploiement où le code **n'utilise plus** la colonne, puis un second déploiement qui la supprime.

### Ajouter une nouvelle table

Le cas sans risque : `CREATE TABLE` n'affecte rien d'existant.

## Règles d'or

1. **Ne modifie jamais une migration déjà appliquée en prod.** Diesel ne la rejouera pas. Écris une nouvelle migration.
2. **Relis le SQL entre `make migration` et `make migrate`**, jamais après.
3. **Teste sur une copie des données de prod** avant tout déploiement qui touche à la structure.
4. **`make backup-prod` avant chaque déploiement** (déjà inclus dans `make deploy-prod`).
5. **Ne lance jamais `make fclean` ni `make re` en prod** : ils suppriment le dossier de la base.
6. **Garde `backups/` hors de git** : un dump contient des emails et des hashes de mots de passe.
7. En cas de doute, **demande une relecture** (section suivante).

## Faire relire ses migrations par une IA

Génère un fichier avec tes migrations et ton schéma :

```bash
for f in migrations/*/*.sql src/schema.rs; do
  echo "--- FILE: $f ---"; cat "$f"; echo
done > review_migrations.txt
```

(Ajoute `review_migrations.txt` à ton `.gitignore`.)

Donne ce fichier à l'IA avec ce prompt :

```
Voici les migrations Diesel de mon projet (Postgres 16) et mon schema.rs.
La base de PRODUCTION contient déjà des données réelles (tables non vides).
Regarde surtout la dernière migration, non encore appliquée en prod.

1. Explique en français simple ce que fait chaque ligne de up.sql.
2. Dis-moi si elle peut ÉCHOUER sur des tables non vides
   (NOT NULL sans DEFAULT, UNIQUE avec doublons, changement de type, clés étrangères...).
3. Dis-moi si elle peut faire PERDRE des données
   (DROP, un renommage vu comme DROP + ADD, changement de type destructif...).
4. Vérifie que down.sql annule bien up.sql.
5. Propose un SQL corrigé si besoin, et une requête pour vérifier les données
   avant de l'appliquer.
```

**Limite à connaître :** une IA peut se tromper sur du SQL, y compris sur ses propres corrections. Sa relecture t'aide à repérer les risques, mais seul le test sur une copie des données de prod (étape 2) prouve que la migration passe.