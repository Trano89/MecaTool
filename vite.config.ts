/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri sert le frontend depuis un port fixe en developpement et depuis des
// fichiers statiques en production. Les deux modes doivent produire exactement
// la meme application : aucune logique ne depend de l'environnement.
export default defineConfig({
  plugins: [react()],

  // Tauri affiche ses propres erreurs de compilation Rust dans le terminal ;
  // les effacer les rendrait invisibles.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Le rechargement a chaud n'a rien a surveiller cote Rust : Tauri
      // recompile lui-meme.
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },

  build: {
    // Cible alignee sur le moteur de rendu embarque par Tauri v2.
    target: "chrome105",
    sourcemap: false,
  },

  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
