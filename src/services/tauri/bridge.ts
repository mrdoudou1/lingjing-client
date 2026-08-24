type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

declare global {
  interface Window { __TAURI_INVOKE__?: TauriInvoke }
}

export const tauriBridge = {
  available: () => typeof window !== 'undefined' && typeof window.__TAURI_INVOKE__ === 'function',
  invoke<T>(command: string, args?: Record<string, unknown>) {
    if (!window.__TAURI_INVOKE__) return Promise.reject(new Error('Tauri bridge unavailable'))
    return window.__TAURI_INVOKE__<T>(command, args)
  },
}
