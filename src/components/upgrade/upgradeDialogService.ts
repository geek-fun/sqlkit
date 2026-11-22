import type { PaidFeature } from '../../common'

export type UpgradeDialogOptions = { coverCta?: boolean }

type OpenUpgradeDialogFn = ((feature?: PaidFeature, options?: UpgradeDialogOptions) => void) | null

let openUpgradeDialogFn: OpenUpgradeDialogFn = null

export function registerUpgradeDialog(fn: OpenUpgradeDialogFn): void {
  openUpgradeDialogFn = fn
}

export function openUpgradeDialog(feature?: PaidFeature, options?: UpgradeDialogOptions): void {
  if (openUpgradeDialogFn) {
    openUpgradeDialogFn(feature, options)
  }
}
