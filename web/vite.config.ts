import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // The WASM glue loads its .wasm via `?url`, so no extra plugin is needed.
  build: {
    target: 'es2020',
    chunkSizeWarningLimit: 4000,
  },
})
