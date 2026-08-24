import type { Asset, GenerationJob } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'

export interface AssetRepository {
  list(): Promise<Asset[]>
  save(asset: Asset): Promise<void>
  remove(id: string): Promise<void>
  usage(): Promise<number>
}

export class PersistentAssetRepository implements AssetRepository {
  private assets: Asset[] = []
  private loaded = false

  private async ensureLoaded() {
    if (!this.loaded) {
      this.assets = (await appPersistence.get<Asset[]>('assets')) ?? []
      this.loaded = true
    }
  }

  async list() { await this.ensureLoaded(); return [...this.assets].sort((a, b) => b.createdAt.localeCompare(a.createdAt)) }
  async save(asset: Asset) { await this.ensureLoaded(); this.assets = [asset, ...this.assets.filter(item => item.id !== asset.id)]; await appPersistence.set('assets', this.assets) }
  async remove(id: string) { await this.ensureLoaded(); this.assets = this.assets.filter(asset => asset.id !== id); await appPersistence.set('assets', this.assets) }
  async usage() { const assets = await this.list(); return assets.reduce((total, asset) => total + asset.sizeBytes, 0) }
}
export const assetRepository = new PersistentAssetRepository()
export function assetFromJob(job: GenerationJob, mimeType = 'video/mp4'): Asset {
  return { id: crypto.randomUUID(), jobId: job.id, kind: job.kind === 'video' ? 'video' : job.kind === 'tts' || job.kind === 'stt' ? 'audio' : 'image', mimeType, localPath: '', sizeBytes: 0, createdAt: new Date().toISOString() }
}
