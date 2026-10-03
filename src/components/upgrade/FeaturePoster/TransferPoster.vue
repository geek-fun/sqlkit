<script lang="ts" setup>
import {
  ArrowLeftRight,
  ArrowRightLeft,
  Check,
  Database,
  FileSpreadsheet,
  Lock,
  Zap,
} from 'lucide-vue-next'
import { ProgressiveBlur } from '../effects'
</script>

<template>
  <div class="poster" aria-hidden="true">
    <div class="poster__bar">
      <span class="poster__dot" />
      <span class="poster__dot" />
      <span class="poster__dot" />
      <span class="poster__pill">{{ $t('transfer.title') }}</span>
      <span class="poster__chip">CSV · JSONL · SQL</span>
    </div>

    <div class="poster__body">
      <div class="p-wizard">
        <div class="p-steps">
          <div class="p-tabs">
            <span class="p-tab p-tab--active">
              <ArrowRightLeft class="p-tab-ic" />
              {{ $t('transfer.tabs.export') }}
            </span>
            <span class="p-tab">
              <ArrowLeftRight class="p-tab-ic" />
              {{ $t('transfer.tabs.import') }}
            </span>
          </div>

          <div class="p-step">
            <div class="p-step-head">
              <Database class="p-step-ic" />
              <span class="p-step-title">{{ $t('transfer.export.step.source') }}</span>
              <span class="p-step-badge">STEP 1</span>
            </div>
            <div class="p-fields">
              <div class="p-field">
                <span class="p-field-label">{{ $t('transfer.connection.label') }}</span>
                <span class="p-field-value">pg-production</span>
              </div>
              <div class="p-field">
                <span class="p-field-label">{{ $t('transfer.export.scope') }}</span>
                <span class="p-field-value">{{ $t('plan.poster.transfer.scopeDatabase') }}</span>
              </div>
            </div>
            <div class="p-indicator">
              <span class="p-indicator-badge">public</span>
              <span class="p-indicator-text">orders · customers · payments</span>
            </div>
          </div>

          <div class="p-step">
            <div class="p-step-head">
              <FileSpreadsheet class="p-step-ic" />
              <span class="p-step-title">{{ $t('transfer.export.step.formatOutput') }}</span>
              <span class="p-step-badge">STEP 2</span>
            </div>
            <div class="p-formats">
              <span class="p-format p-format--on">CSV</span>
              <span class="p-format">JSONL</span>
              <span class="p-format">SQL</span>
              <span class="p-format">Parquet</span>
            </div>
            <div class="p-field">
              <span class="p-field-label">{{ $t('transfer.export.delimiter') }}</span>
              <span class="p-field-value">, · {{ $t('transfer.export.includeHeader') }}</span>
            </div>
          </div>
        </div>

        <div class="p-panel">
          <div class="p-panel-head">
            <Zap class="p-panel-ic" />
            <span class="p-panel-title">{{ $t('transfer.tasks.title') }}</span>
            <span class="p-chip">64%</span>
          </div>
          <div class="p-task">
            <div class="p-task-row">
              <span class="p-task-name">public.orders → orders.csv</span>
              <span class="p-task-done">
                <Check class="p-task-check" />
                48,102
              </span>
            </div>
            <div class="p-task-track">
              <i class="p-task-fill p-task-fill--done" style="width: 100%" />
            </div>
          </div>
          <div class="p-task">
            <div class="p-task-row">
              <span class="p-task-name">public.customers → customers.csv</span>
              <span class="p-task-count">64%</span>
            </div>
            <div class="p-task-track">
              <i class="p-task-fill p-task-fill--running" style="width: 64%" />
            </div>
          </div>
          <p class="p-note">
            12,400 / 38,000 {{ $t('transfer.tasks.rows') }}
          </p>
        </div>
      </div>
    </div>

    <div class="poster__cta">
      <slot name="cta" />
    </div>

    <ProgressiveBlur />
    <div class="poster__lock">
      <Lock class="poster__lock-icon" />
      {{ $t('plan.state.ultimate') }}
    </div>
  </div>
</template>

<style scoped>
.poster {
  position: relative;
  overflow: hidden;
  border-radius: 14px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--card));
  box-shadow: 0 24px 64px -20px rgba(0, 0, 0, 0.28);
}

.poster__bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  border-bottom: 1px solid hsl(var(--border));
  background-color: hsl(var(--muted) / 0.5);
}

.poster__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: hsl(var(--border));
}

.poster__pill {
  margin-left: 8px;
  padding: 2px 11px;
  border-radius: 999px;
  border: 1px solid hsl(var(--border));
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: hsl(var(--muted-foreground));
}

.poster__chip {
  margin-left: auto;
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid hsl(var(--primary) / 0.35);
  color: hsl(var(--primary));
  font-size: 10px;
  font-weight: 600;
}

.poster__body {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 14px;
  background-color: hsl(var(--background));
}

.p-wizard {
  display: flex;
  flex: 1;
  min-height: 0;
  gap: 12px;
  align-items: stretch;
}

/* ── steps column — mirrors the real tabs + TransferStepCard stack ── */
.p-steps {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  gap: 8px;
}

.p-tabs {
  display: flex;
  padding: 3px;
  border-radius: 9px;
  border: 1px solid hsl(var(--border) / 0.4);
  background-color: hsl(var(--muted) / 0.5);
}

.p-tab {
  display: inline-flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 5px 0;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 500;
  color: hsl(var(--muted-foreground));
}

.p-tab--active {
  background-color: hsl(var(--background));
  border: 1px solid hsl(var(--border) / 0.4);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
  color: hsl(var(--primary));
  font-weight: 600;
}

.p-tab-ic {
  width: 12px;
  height: 12px;
}

.p-step {
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: center;
  border: 1px solid hsl(var(--border) / 0.4);
  border-radius: 10px;
  padding: 10px 12px;
  background-color: hsl(var(--card));
}

.p-step-head {
  display: flex;
  align-items: center;
  gap: 7px;
  padding-bottom: 8px;
}

.p-step-ic {
  width: 14px;
  height: 14px;
  color: hsl(152 60% 38%);
  flex-shrink: 0;
}

.dark .p-step-ic {
  color: hsl(152 55% 50%);
}

.p-step-title {
  font-size: 11.5px;
  font-weight: 600;
  color: hsl(var(--foreground));
}

.p-step-badge {
  margin-left: auto;
  padding: 1px 7px;
  border-radius: 999px;
  border: 1px solid hsl(var(--border));
  color: hsl(var(--muted-foreground));
  font-size: 8.5px;
  font-weight: 700;
  letter-spacing: 0.06em;
}

.p-fields {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.p-field {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 5px 8px;
  border: 1px solid hsl(var(--border) / 0.7);
  border-radius: 7px;
  background-color: hsl(var(--muted) / 0.35);
}

.p-field-label {
  font-size: 8.5px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: hsl(var(--muted-foreground));
}

.p-field-value {
  font-size: 10.5px;
  font-weight: 600;
  color: hsl(var(--foreground));
  font-variant-numeric: tabular-nums;
}

.p-indicator {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 8px;
}

.p-indicator-badge {
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid hsl(var(--border) / 0.6);
  background-color: hsl(var(--muted) / 0.5);
  color: hsl(var(--muted-foreground));
  font-size: 9px;
  font-weight: 700;
  font-family: ui-monospace, 'SF Mono', SFMono-Regular, Menlo, monospace;
  white-space: nowrap;
}

.p-indicator-text {
  font-size: 9px;
  color: hsl(var(--muted-foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.p-formats {
  display: flex;
  gap: 5px;
}

.p-format {
  flex: 1;
  text-align: center;
  padding: 5px 0;
  border: 1px solid hsl(var(--border) / 0.5);
  border-radius: 7px;
  font-size: 10px;
  font-weight: 600;
  color: hsl(var(--muted-foreground));
}

.p-format--on {
  border-color: hsl(var(--primary) / 0.6);
  box-shadow: 0 0 0 1px hsl(var(--primary) / 0.2);
  background-color: hsl(var(--primary) / 0.04);
  color: hsl(var(--primary));
}

/* ── task panel — mirrors the real TaskManagerPanel / TaskCard ── */
.p-panel {
  display: flex;
  width: 300px;
  flex-shrink: 0;
  flex-direction: column;
  min-width: 0;
  border: 1px solid hsl(var(--border) / 0.4);
  border-radius: 10px;
  background-color: hsl(var(--card));
  padding: 11px 13px;
  gap: 10px;
}

.p-panel-head {
  display: flex;
  align-items: center;
  gap: 7px;
}

.p-panel-ic {
  width: 14px;
  height: 14px;
  color: hsl(38 92% 50%);
}

.p-panel-title {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: hsl(var(--foreground));
}

.p-chip {
  margin-left: auto;
  padding: 1px 8px;
  border-radius: 999px;
  background-color: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
  font-size: 9.5px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.p-task {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.p-task-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.p-task-name {
  font-size: 10.5px;
  font-family: ui-monospace, 'SF Mono', SFMono-Regular, Menlo, monospace;
  color: hsl(var(--foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.p-task-count {
  margin-left: auto;
  font-size: 9.5px;
  color: hsl(var(--muted-foreground));
  font-variant-numeric: tabular-nums;
}

.p-task-done {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 9.5px;
  font-weight: 600;
  color: hsl(var(--method-post));
  font-variant-numeric: tabular-nums;
}

.p-task-check {
  width: 10px;
  height: 10px;
}

.p-task-track {
  height: 6px;
  border-radius: 999px;
  background-color: hsl(var(--muted));
  overflow: hidden;
}

.p-task-fill {
  display: block;
  height: 100%;
  border-radius: inherit;
}

.p-task-fill--done {
  background-color: hsl(var(--method-post) / 0.8);
}

.p-task-fill--running {
  background-color: hsl(var(--primary) / 0.7);
  background-image: linear-gradient(
    45deg,
    rgba(255, 255, 255, 0.28) 25%,
    transparent 25%,
    transparent 50%,
    rgba(255, 255, 255, 0.28) 50%,
    rgba(255, 255, 255, 0.28) 75%,
    transparent 75%
  );
  background-size: 12px 12px;
  animation: p-stripes 0.9s linear infinite;
}

.p-note {
  margin: 0;
  font-size: 9.5px;
  color: hsl(var(--muted-foreground));
  font-variant-numeric: tabular-nums;
}

.poster__lock {
  position: absolute;
  left: 24px;
  bottom: 24px;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 13px;
  border-radius: 999px;
  border: 1px solid hsl(var(--primary) / 0.4);
  background-color: hsl(var(--background) / 0.75);
  backdrop-filter: blur(8px);
  font-size: 11.5px;
  font-weight: 600;
  color: hsl(var(--primary));
}

.poster__lock-icon {
  width: 12px;
  height: 12px;
}

@keyframes p-stripes {
  to {
    background-position: 17px 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .p-task-fill--running {
    animation: none;
  }
}

/* CTA slot: laid out inside the poster's own flex column so the button is
   structurally pinned to the window bottom — immune to containing-block
   resolution and content height changes. */
.poster__cta {
  position: relative;
  z-index: 2;
  display: flex;
  justify-content: center;
  padding: 18px 0 20px;
}
</style>
