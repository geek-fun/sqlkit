import type { ClassValue } from 'clsx'
import { clsx } from 'clsx'

export const cn = (...inputs: ClassValue[]) => clsx(inputs)
