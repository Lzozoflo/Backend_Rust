// `embed_migrations!` embarque les fichiers SQL à la compilation : sans ça, Cargo ne
// recompile pas quand on AJOUTE un dossier de migration (cargo watch compris).
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
