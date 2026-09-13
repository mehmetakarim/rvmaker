import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// Tauri bir sabit port bekler; Vite'ın ekranı temizlemesi Rust loglarını yutuyor.
export default defineConfig({
  plugins: [vue()],
  define: {
    // Kenar çubuğunda gösterilir; hangi paketin çalıştığını ayırt etmeyi sağlar.
    __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
  },
  clearScreen: false,
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/Design/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "safari15",
    minify: "esbuild",
    sourcemap: false,
  },
});
