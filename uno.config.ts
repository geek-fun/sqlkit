import { defineConfig, presetIcons, transformerDirectives, transformerVariantGroup } from 'unocss'
import { presetWind4 } from '@unocss/preset-wind4'
import presetAnimations from 'unocss-preset-animations'

export default defineConfig({
  presets: [
    presetWind4({
      preflights: {
        reset: true,
        theme: 'on-demand',
        property: true,
      },
    }),
    presetIcons({
      scale: 1.2,
      cdn: 'https://esm.sh/',
    }),
    presetAnimations(),
  ],
  transformers: [transformerDirectives(), transformerVariantGroup()],
  content: {
    filesystem: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  },
  theme: {
    colors: {
      border: 'hsl(var(--border))',
      input: 'hsl(var(--input))',
      ring: 'hsl(var(--ring))',
      background: 'hsl(var(--background))',
      foreground: 'hsl(var(--foreground))',
      primary: {
        DEFAULT: 'hsl(var(--primary))',
        foreground: 'hsl(var(--primary-foreground))',
      },
      secondary: {
        DEFAULT: 'hsl(var(--secondary))',
        foreground: 'hsl(var(--secondary-foreground))',
      },
      destructive: {
        DEFAULT: 'hsl(var(--destructive))',
        foreground: 'hsl(var(--destructive-foreground))',
      },
      muted: {
        DEFAULT: 'hsl(var(--muted))',
        foreground: 'hsl(var(--muted-foreground))',
      },
      accent: {
        DEFAULT: 'hsl(var(--accent))',
        foreground: 'hsl(var(--accent-foreground))',
      },
      popover: {
        DEFAULT: 'hsl(var(--popover))',
        foreground: 'hsl(var(--popover-foreground))',
      },
      card: {
        DEFAULT: 'hsl(var(--card))',
        foreground: 'hsl(var(--card-foreground))',
      },
    },
    radius: {
      lg: 'var(--radius)',
      md: 'calc(var(--radius) - 2px)',
      sm: 'calc(var(--radius) - 4px)',
    },
    animation: {
      keyframes: {
        'accordion-down': '{from{height:0}to{height:var(--radix-accordion-content-height)}}',
        'accordion-up': '{from{height:var(--radix-accordion-content-height)}to{height:0}}',
      },
      durations: {
        'accordion-down': '0.2s',
        'accordion-up': '0.2s',
      },
      ease: {
        'accordion-down': 'ease-out',
        'accordion-up': 'ease-out',
      },
    },
  },
  shortcuts: {
    'flex-center': 'flex items-center justify-center',
    'flex-col-center': 'flex flex-col items-center justify-center',
  },
  safelist: [
    'bg-primary',
    'text-primary-foreground',
    'hover:bg-primary/90',
    'bg-destructive',
    'text-destructive-foreground',
    'hover:bg-destructive/90',
    'bg-secondary',
    'text-secondary-foreground',
    'hover:bg-secondary/80',
    'bg-accent',
    'text-accent-foreground',
    'hover:bg-accent',
    'hover:text-accent-foreground',
    'text-primary',
    'bg-popover',
    'text-popover-foreground',
    'bg-destructive/10',
    'text-destructive',
    'hover:bg-destructive/10',
    'hover:text-destructive-foreground',
  ],
})
