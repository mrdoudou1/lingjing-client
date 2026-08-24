export interface PersistenceAdapter { get<T>(key: string): Promise<T | null>; set<T>(key: string, value: T): Promise<void> }

export class MemoryPersistence implements PersistenceAdapter {
  private values = new Map<string, unknown>()
  async get<T>(key: string) { return (this.values.get(key) as T | undefined) ?? null }
  async set<T>(key: string, value: T) { this.values.set(key, value) }
}
export const persistence = new MemoryPersistence()

/** Browser persistence is limited to non-sensitive product state. API keys never enter this adapter. */
export class BrowserPersistence implements PersistenceAdapter {
  async get<T>(key: string) {
    try {
      const value = window.localStorage.getItem(`lingjing:${key}`)
      return value ? JSON.parse(value) as T : null
    } catch { return null }
  }
  async set<T>(key: string, value: T) {
    window.localStorage.setItem(`lingjing:${key}`, JSON.stringify(value))
  }
}

export const appPersistence: PersistenceAdapter = typeof window === 'undefined' ? persistence : new BrowserPersistence()
