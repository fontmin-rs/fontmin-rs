import { defineConfig } from 'tsdown'

export default defineConfig({
  clean: true,
  dts: {
    tsgo: true,
  },
  entry: [
    'src/index.ts',
    'src/plugins.ts',
    'src/presets.ts',
    'src/compat.ts',
    'src/vinyl.ts',
    'src/cli.mjs',
  ],
  platform: 'node',
})
