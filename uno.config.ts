import { defineConfig, presetWind4 } from 'unocss'
import { presetShadcn } from 'unocss-preset-shadcn'

export default defineConfig({
  presets: [
    presetWind4(),
    presetShadcn({
      color: 'slate',
    }),
  ],
  content: {
    pipeline: {
      include: [
        // Default includes
        /\.(vue|svelte|[jt]sx|mdx?|astro|elm|php|phtml|html)($|\?)/,
        // Include src directory
        'src/**/*.{vue,js,ts,jsx,tsx}',
      ],
    },
  },
})
