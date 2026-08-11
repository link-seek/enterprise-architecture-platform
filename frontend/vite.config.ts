import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    host: '0.0.0.0',
    proxy: {
      '/api': {
        target: `http://${process.env.VITE_BACKEND_HOST || 'localhost'}:8080`,
        changeOrigin: true,
      },
      '/graphql': {
        target: `http://${process.env.VITE_BACKEND_HOST || 'localhost'}:8080`,
        changeOrigin: true,
      },
      '/health': {
        target: `http://${process.env.VITE_BACKEND_HOST || 'localhost'}:8080`,
        changeOrigin: true,
      },
    },
  },
})
