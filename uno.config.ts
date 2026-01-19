import { defineConfig, presetWind4 } from 'unocss'
import { presetShadcn } from 'unocss-preset-shadcn'

export default defineConfig({
  presets: [
    presetWind4(),
    presetShadcn(),
  ],
})
