//! Pont entre le moteur Mecatol et l'interface.
//!
//! Cette couche ne calcule rien. Elle lit l'entree, appelle le moteur, et rend
//! le resultat tel quel. Toute regle normative, toute conversion d'unite, toute
//! decision de geometrie reste en amont : le frontend recoit des valeurs deja
//! calculees et deja mises en forme.
//!
//! # Pourquoi les commandes vivent dans un sous-module
//!
//! `#[tauri::command]` genere une macro marquee `#[macro_export]`, donc publiee
//! a la racine du crate, puis un `pub use` du meme nom dans le module courant.
//! Place a la racine, le second entre en collision avec le premier (E0255).
//! Le sous-module `commands` n'est donc pas une preference de style : c'est la
//! seule disposition qui compile.

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::analyse,
            commands::rescale_diagram,
            commands::engine_info,
            commands::general_tolerances,
            commands::compare,
            commands::dimension_chain
        ])
        .run(tauri::generate_context!())
        .expect("le lancement de Mecatol a échoué");
}
