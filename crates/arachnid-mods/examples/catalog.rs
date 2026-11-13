fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = arachnid_mods::bundled_catalog().map_err(|error| error.code)?;
    println!("Catalogue embarqué informatif — aucune installation autorisée");
    for group in &catalog.groups {
        println!("Groupe {} : {}", group.id, group.members.join(" + "));
        println!("  Sélection initiale du groupe : {}", group.default_enabled);
    }
    for recipe in catalog.entries {
        println!("{} — {}", recipe.id, recipe.name);
        println!("  Source : {}", recipe.source);
        println!(
            "  Sélection souhaitée par défaut : {} (pas un état installé)",
            recipe.default_enabled
        );
        println!("  Preuves manquantes : {:?}", recipe.missing_evidence());
    }
    Ok(())
}
