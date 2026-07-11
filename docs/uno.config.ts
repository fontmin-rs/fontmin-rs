import { defineConfig, presetIcons, presetWind3 } from 'unocss'

export default defineConfig({
  preflights: [],
  presets: [
    presetWind3(),
    presetIcons({
      collections: {
        lucide: () =>
          import('@iconify-json/lucide/icons.json').then(
            module => module.default,
          ),
      },
    }),
  ],
})
