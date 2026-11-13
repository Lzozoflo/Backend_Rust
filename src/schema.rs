// Schéma Diesel. Modifie ce fichier, puis lance `make migration name=<nom>` :
// Diesel compare ce fichier à la base et écrit la migration SQL pour toi.
// `make migrate` l'applique puis réécrit ce fichier depuis la base.

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
