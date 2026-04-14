const MIN_LOADING_TIME = 500

export async function withMinLoadingTime<T>(fn: () => Promise<T>, minDuration: number = MIN_LOADING_TIME): Promise<T> {
  const startTime = Date.now()

  const result = await fn()

  const elapsed = Date.now() - startTime
  if (elapsed < minDuration) {
    await new Promise(resolve => setTimeout(resolve, minDuration - elapsed))
  }

  return result
}

export function useMinLoadingTime() {
  return { withMinLoadingTime }
}
