import { defineConfig, presetIcons, presetWind3 } from 'unocss'

export default defineConfig({
  preflights: [],
  presets: [
    presetWind3(),
    presetIcons({
      collections: {
        lucide: async () => {
          const module = await import('@iconify-json/lucide/icons.json')
          return module.default
        },
      },
    }),
  ],
})
