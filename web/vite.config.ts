import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // Relative asset paths so the built bundle works from any base (GitHub Pages,
  // sub-directory hosting, file://). The WASM glue loads its .wasm via `?url`.
  base: './',
  build: {
    target: 'es2020',
    chunkSizeWarningLimit: 4000,
  },
})
