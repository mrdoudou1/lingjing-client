import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { listen as tauriListen } from '@tauri-apps/api/event'

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

declare global {
  interface Window { __TAURI_INVOKE__?: TauriInvoke; __TAURI_INTERNALS__?: unknown }
}

function hasTauriRuntime() {
  return typeof window !== 'undefined' && (Boolean(window.__TAURI_INTERNALS__) || typeof window.__TAURI_INVOKE__ === 'function')
}

export const tauriBridge = {
  available: hasTauriRuntime,
  invoke<T>(command: string, args?: Record<string, unknown>) {
    if (window.__TAURI_INVOKE__) return window.__TAURI_INVOKE__<T>(command, args)
    if (hasTauriRuntime()) return tauriInvoke<T>(command, args)
    return Promise.reject(new Error('Tauri bridge unavailable'))
  },
  listen<T>(event: string, handler: (payload: T) => void) {
    if (!hasTauriRuntime()) return Promise.reject(new Error('Tauri bridge unavailable'))
    return tauriListen<T>(event, eventPayload => handler(eventPayload.payload))
  },
}
