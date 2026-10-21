import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Badge } from './Badge.vue'

export const badgeVariants = cva(
  'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold',
  {
    variants: {
      variant: {
        default:
          'border-transparent bg-primary text-primary-foreground',
        secondary:
          'border-transparent bg-secondary text-secondary-foreground',
        destructive:
          'border-destructive-border bg-destructive-muted text-destructive-text',
        outline: 'text-foreground',
        success:
          'border-success-border bg-success-muted text-success',
        warning:
          'border-warning-border bg-warning-muted text-warning',
      },
    },
    defaultVariants: {
      variant: 'secondary',
    },
  },
)

export type BadgeVariants = VariantProps<typeof badgeVariants>
