import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://v2.tauri.app/start/frontend/vite/
const isTauri = !!process.env.TAURI_ENV_FAMILY;

export default defineConfig({
  plugins: [react(), tailwindcss()],

  // Prevent Vite from obscuring Rust errors
  clearScreen: false,

  server: {
    port: 5173,
    strictPort: true,
    // Tauri needs the dev server to be available on all interfaces for mobile
    host: isTauri ? '0.0.0.0' : 'localhost',
  },

  // Env prefix for Tauri env vars
  envPrefix: ['VITE_', 'TAURI_ENV_*'],

  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS/Linux
    target: 'esnext',
    // Disable minification for debug builds
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})
