export interface PersistenceAdapter { get<T>(key: string): Promise<T | null>; set<T>(key: string, value: T): Promise<void> }

export class MemoryPersistence implements PersistenceAdapter {
  private values = new Map<string, unknown>()
  async get<T>(key: string) { return (this.values.get(key) as T | undefined) ?? null }
  async set<T>(key: string, value: T) { this.values.set(key, value) }
}
export const persistence = new MemoryPersistence()
