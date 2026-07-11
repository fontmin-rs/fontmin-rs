import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@fontmin-rs/wasm': fileURLToPath(
        new URL('../wasm/fontmin/src/index.ts', import.meta.url),
      ),
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['.vitepress/**/*.test.ts'],
  },
})
