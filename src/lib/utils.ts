import type { ClassValue } from 'clsx'
import { clsx } from 'clsx'
import { extendTailwindMerge } from 'tailwind-merge'

const mergeClasses = extendTailwindMerge({
  extend: {
    classGroups: {
      'font-size': ['text-caption', 'text-body'],
    },
  },
})

export const cn = (...inputs: ClassValue[]): string => mergeClasses(clsx(inputs))
