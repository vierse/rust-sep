import { defineConfig } from "vite"
import react from "@vitejs/plugin-react-swc"
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'


const __dirname = dirname(fileURLToPath(import.meta.url))

// https://vite.dev/config/
export default defineConfig({
  server: {
    host: "0.0.0.0",
    port: 5173,
    proxy: {
      // route to axum (dev only)
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
      "/r": {
        target: "http://localhost:3000",
        changeOrigin: true,
      }
    },
  },
  plugins: [react()],
 
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        metrics: resolve(__dirname, 'metrics/index.html'),
      },
    },
  },
});
