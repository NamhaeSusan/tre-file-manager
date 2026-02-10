import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  server: {
    port: 3000,
    proxy: {
      '/api': 'http://localhost:9090',
      '/ws': { target: 'ws://localhost:9090', ws: true },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
