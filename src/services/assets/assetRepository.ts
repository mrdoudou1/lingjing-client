import type { Asset, GenerationJob } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'
import { tauriBridge } from '../tauri/bridge'

type RustAsset = {
  id: string
  job_id?: string
  kind: Asset['kind']
  mime_type: string
  local_path: string
  thumbnail_path?: string
  size_bytes: number
  favorite?: boolean
  created_at: string
}

const fromRustAsset = (asset: RustAsset): Asset => ({
  id: asset.id,
  jobId: asset.job_id,
  kind: asset.kind,
  mimeType: asset.mime_type,
  localPath: asset.local_path,
  thumbnailPath: asset.thumbnail_path,
  sizeBytes: asset.size_bytes,
  favorite: asset.favorite,
  createdAt: asset.created_at,
})

const toRustAsset = (asset: Asset): RustAsset => ({
  id: asset.id,
  job_id: asset.jobId,
  kind: asset.kind,
  mime_type: asset.mimeType,
  local_path: asset.localPath,
  thumbnail_path: asset.thumbnailPath,
  size_bytes: asset.sizeBytes,
  favorite: asset.favorite,
  created_at: asset.createdAt,
})

export interface AssetRepository {
  list(): Promise<Asset[]>
  save(asset: Asset): Promise<void>
  remove(id: string): Promise<void>
  toggleFavorite(id: string): Promise<void>
  openLocation(id: string): Promise<string | undefined>
  usage(): Promise<number>
}

export class PersistentAssetRepository implements AssetRepository {
  private assets: Asset[] = []
  private loaded = false

  private async ensureLoaded() {
    if (!this.loaded) {
      if (tauriBridge.available()) {
        try {
          const stored = await tauriBridge.invoke<RustAsset[]>('asset_list')
          this.assets = stored.map(fromRustAsset)
          this.loaded = true
          return
        } catch { /* browser persistence remains the fallback */ }
      }
      this.assets = (await appPersistence.get<Asset[]>('assets')) ?? []
      this.loaded = true
    }
  }

  async list() { await this.ensureLoaded(); return [...this.assets].sort((a, b) => b.createdAt.localeCompare(a.createdAt)) }
  async reload() { this.loaded = false; await this.ensureLoaded() }
  async save(asset: Asset) {
    await this.ensureLoaded()
    if (tauriBridge.available()) {
      await tauriBridge.invoke('asset_register', { asset: toRustAsset(asset) })
    } else {
      await appPersistence.set('assets', [asset, ...this.assets.filter(item => item.id !== asset.id)])
    }
    this.assets = [asset, ...this.assets.filter(item => item.id !== asset.id)]
  }
  async remove(id: string) {
    await this.ensureLoaded()
    if (tauriBridge.available()) {
      await tauriBridge.invoke('asset_delete', { id })
    } else {
      await appPersistence.set('assets', this.assets.filter(asset => asset.id !== id))
    }
    this.assets = this.assets.filter(asset => asset.id !== id)
  }
  async toggleFavorite(id: string) {
    await this.ensureLoaded()
    const current = this.assets.find(asset => asset.id === id)
    if (!current) return
    if (tauriBridge.available()) {
      const result = await tauriBridge.invoke<RustAsset | null>('asset_toggle_favorite', { id })
      if (result) this.assets = this.assets.map(asset => asset.id === id ? fromRustAsset(result) : asset)
      return
    }
    const next = this.assets.map(asset => asset.id === id ? { ...asset, favorite: !asset.favorite } : asset)
    this.assets = next
    await appPersistence.set('assets', next)
  }
  async openLocation(id: string) {
    await this.ensureLoaded()
    if (!tauriBridge.available()) return this.assets.find(asset => asset.id === id)?.localPath
    return tauriBridge.invoke<string | null>('asset_open_location', { id }).then(path => path ?? undefined)
  }
  async usage() { const assets = await this.list(); return assets.reduce((total, asset) => total + asset.sizeBytes, 0) }
}
export const assetRepository = new PersistentAssetRepository()
export function assetFromJob(job: GenerationJob, mimeType = 'video/mp4'): Asset {
  return { id: crypto.randomUUID(), jobId: job.id, kind: job.kind === 'video' ? 'video' : job.kind === 'tts' || job.kind === 'stt' ? 'audio' : 'image', mimeType, localPath: '', sizeBytes: 0, createdAt: new Date().toISOString() }
}
