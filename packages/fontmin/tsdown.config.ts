import { defineConfig } from 'tsdown'

export default defineConfig({
  clean: true,
  dts: {
    tsgo: true,
  },
  entry: ['src/index.ts', 'src/plugins.ts', 'src/presets.ts', 'src/compat.ts'],
  platform: 'node',
})
