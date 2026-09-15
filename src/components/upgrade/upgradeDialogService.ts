import type { PaidFeature } from '../../common'

type OpenUpgradeDialogFn = ((feature?: PaidFeature) => void) | null

let openUpgradeDialogFn: OpenUpgradeDialogFn = null

export function registerUpgradeDialog(fn: OpenUpgradeDialogFn): void {
  openUpgradeDialogFn = fn
}

export function openUpgradeDialog(feature?: PaidFeature): void {
  if (openUpgradeDialogFn) {
    openUpgradeDialogFn(feature)
  }
}
