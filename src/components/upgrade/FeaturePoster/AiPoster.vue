<script lang="ts" setup>
import { Check, Lightbulb, Lock, RefreshCw, Repeat, Terminal } from 'lucide-vue-next'
import { ProgressiveBlur } from '../effects'
</script>

<template>
  <div class="poster" aria-hidden="true">
    <div class="poster__bar">
      <span class="poster__dot" />
      <span class="poster__dot" />
      <span class="poster__dot" />
      <span class="poster__pill">{{ $t('dataStudio.title') }}</span>
      <span class="poster__model">SQL</span>
    </div>

    <div class="poster__chat">
      <div class="p-user">
        <p>{{ $t('plan.poster.ai.question') }}</p>
      </div>

      <div class="p-timeline">
        <div class="p-iter">
          <Repeat class="p-ic" />
          <span>{{ $t('plan.poster.ai.loop') }}</span>
        </div>
        <div class="p-row p-row--think">
          <Lightbulb class="p-ic p-ic--pulse" />
          <span class="p-row-label">{{ $t('plan.poster.ai.thinking') }}</span>
          <span class="p-dots"><i /><i /><i /></span>
          <span class="p-badge">2.4s</span>
        </div>
        <div class="p-row p-row--tool">
          <Terminal class="p-ic" />
          <span class="p-tool-name">run_sql</span>
          <span class="p-row-label">{{ $t('plan.poster.ai.toolVerb') }}</span>
          <RefreshCw class="p-spin" />
          <Check class="p-check" />
          <span class="p-chip">142 rows · 38 ms</span>
        </div>
      </div>

      <div class="p-answer">
        <p class="p-answer-text">
          {{ $t('plan.poster.ai.answer') }}
        </p>
        <pre class="p-code">SELECT c.name, SUM(o.total) AS revenue
FROM orders o
JOIN customers c ON c.id = o.customer_id
WHERE o.created_at >= NOW() - INTERVAL '7 days'
  AND o.status = 'failed'
GROUP BY c.name
ORDER BY revenue DESC;</pre>
        <div class="p-table">
          <div class="p-tr p-tr--head">
            <span>name</span>
            <span>revenue</span>
            <span>failed</span>
          </div>
          <div class="p-tr">
            <span>Acme Corp</span>
            <span>$1,290.00</span>
            <span>3</span>
          </div>
          <div class="p-tr">
            <span>Globex</span>
            <span>$845.50</span>
            <span>2</span>
          </div>
          <div class="p-tr">
            <span>Initech</span>
            <span>$402.25</span>
            <span>1</span>
          </div>
        </div>
      </div>
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

.poster__model {
  margin-left: auto;
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid hsl(var(--primary) / 0.35);
  color: hsl(var(--primary));
  font-size: 10px;
  font-weight: 600;
}

.poster__chat {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background-color: hsl(var(--background));
}

/* ── user turn — solid primary bubble, matches the real chat ── */
.p-user {
  align-self: flex-end;
  max-width: 76%;
  padding: 8px 13px;
  border-radius: 12px 12px 4px 12px;
  background-color: hsl(var(--primary));
  color: hsl(var(--primary-foreground));
  font-size: 12px;
  animation: p-user-in 12s linear both;
}

/* ── assistant activity timeline ── */
.p-timeline {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding-left: 16px;
  max-width: 94%;
}

.p-timeline::before {
  content: '';
  position: absolute;
  left: 5px;
  top: 5px;
  bottom: 5px;
  width: 1px;
  background-color: hsl(var(--border));
}

.p-iter {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: hsl(var(--muted-foreground));
  animation: p-step-1 12s linear both;
}

.p-ic {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  color: hsl(var(--muted-foreground));
}

.p-row {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 20px;
}

.p-row--think {
  animation: p-step-1 12s linear both;
}

.p-row--tool {
  animation: p-step-2 12s linear both;
}

.p-row-label {
  font-size: 11px;
  color: hsl(var(--muted-foreground));
}

.p-ic--pulse {
  color: hsl(var(--primary));
  animation: p-pulse 1.6s ease-in-out infinite;
}

.p-dots {
  display: inline-flex;
  gap: 3px;
  animation: p-visible-1 12s linear both;
}

.p-dots i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background-color: hsl(var(--muted-foreground));
  animation: p-dot-pulse 1.1s ease-in-out infinite;
}

.p-dots i:nth-child(2) {
  animation-delay: 0.15s;
}

.p-dots i:nth-child(3) {
  animation-delay: 0.3s;
}

.p-badge {
  padding: 1px 7px;
  border-radius: 999px;
  background-color: hsl(var(--muted));
  color: hsl(var(--muted-foreground));
  font-size: 9.5px;
  font-variant-numeric: tabular-nums;
  animation: p-visible-2 12s linear both;
}

.p-tool-name {
  padding: 1px 7px;
  border-radius: 6px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--muted) / 0.5);
  font-family: ui-monospace, 'SF Mono', SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  color: hsl(var(--foreground));
}

.p-spin {
  width: 12px;
  height: 12px;
  color: hsl(var(--primary));
  animation:
    p-visible-1 12s linear both,
    p-rotate 1.1s linear infinite;
}

.p-check {
  width: 12px;
  height: 12px;
  color: hsl(var(--method-post));
  animation: p-check-flow 12s linear both;
}

.p-chip {
  padding: 1px 8px;
  border-radius: 999px;
  background-color: hsl(var(--method-post) / 0.12);
  color: hsl(var(--method-post));
  font-size: 9.5px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  animation: p-visible-2 12s linear both;
}

/* ── final answer ── */
.p-answer {
  display: flex;
  flex-direction: column;
  gap: 9px;
  max-width: 94%;
  animation: p-answer-flow 12s linear both;
}

.p-answer-text {
  margin: 0;
  font-size: 12px;
  color: hsl(var(--foreground));
}

.p-code {
  margin: 0;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid hsl(var(--border));
  background-color: hsl(var(--muted) / 0.55);
  font-family: ui-monospace, 'SF Mono', SFMono-Regular, Menlo, monospace;
  font-size: 10.5px;
  line-height: 1.55;
  color: hsl(var(--foreground));
  overflow: hidden;
  animation: p-code-flow 12s linear both;
}

.p-table {
  border: 1px solid hsl(var(--border));
  border-radius: 8px;
  overflow: hidden;
  font-size: 11px;
}

.p-tr {
  display: grid;
  grid-template-columns: 1.4fr 1fr 0.7fr;
}

.p-tr > span {
  padding: 5px 10px;
  color: hsl(var(--muted-foreground));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

.p-tr--head > span {
  background-color: hsl(var(--muted) / 0.6);
  font-weight: 600;
  color: hsl(var(--foreground));
}

.p-tr + .p-tr > span {
  border-top: 1px solid hsl(var(--border));
}

.p-answer .p-tr {
  animation: p-code-flow 12s linear both;
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

/* ── one-shot scenario build-up: ask → think → query → answer, then hold ── */
@keyframes p-user-in {
  0% {
    opacity: 0;
    transform: translateY(8px);
  }
  4% {
    opacity: 1;
    transform: none;
  }
  100% {
    opacity: 1;
    transform: none;
  }
}

@keyframes p-step-1 {
  0%,
  6% {
    opacity: 0;
    transform: translateY(8px);
  }
  9% {
    opacity: 1;
    transform: none;
  }
  100% {
    opacity: 1;
    transform: none;
  }
}

@keyframes p-step-2 {
  0%,
  23% {
    opacity: 0;
    transform: translateY(8px);
  }
  26% {
    opacity: 1;
    transform: none;
  }
  100% {
    opacity: 1;
    transform: none;
  }
}

@keyframes p-answer-flow {
  0%,
  46% {
    opacity: 0;
    transform: translateY(8px);
  }
  50% {
    opacity: 1;
    transform: none;
  }
  100% {
    opacity: 1;
    transform: none;
  }
}

@keyframes p-code-flow {
  0%,
  53% {
    opacity: 0;
  }
  57% {
    opacity: 1;
  }
  100% {
    opacity: 1;
  }
}

/* thinking dots pulse, then hand over to the duration badge */
@keyframes p-visible-1 {
  0%,
  6% {
    opacity: 0;
  }
  9% {
    opacity: 1;
  }
  20% {
    opacity: 1;
  }
  23%,
  100% {
    opacity: 0;
  }
}

@keyframes p-visible-2 {
  0%,
  19% {
    opacity: 0;
  }
  23% {
    opacity: 1;
  }
  100% {
    opacity: 1;
  }
}

@keyframes p-check-flow {
  0%,
  36% {
    opacity: 0;
    transform: scale(0.6);
  }
  40% {
    opacity: 1;
    transform: scale(1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes p-dot-pulse {
  0%,
  100% {
    opacity: 0.35;
  }
  50% {
    opacity: 1;
  }
}

@keyframes p-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.45;
  }
}

@keyframes p-rotate {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .p-user,
  .p-iter,
  .p-row,
  .p-dots,
  .p-badge,
  .p-spin,
  .p-check,
  .p-chip,
  .p-answer,
  .p-code,
  .p-answer .p-tr,
  .p-ic--pulse {
    animation: none;
  }

  .p-dots,
  .p-spin {
    display: none;
  }
}
</style>
