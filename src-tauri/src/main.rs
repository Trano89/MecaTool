// La console Windows ne doit pas s'ouvrir derrière la fenêtre en production.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mecatol_app_lib::run()
}
