import type { Asset, GenerationJob } from '../../types/domain'

export interface AssetRepository {
  list(): Promise<Asset[]>
  save(asset: Asset): Promise<void>
}

export class InMemoryAssetRepository implements AssetRepository {
  private assets: Asset[] = []
  async list() { return this.assets }
  async save(asset: Asset) { this.assets = [asset, ...this.assets] }
}
export const assetRepository = new InMemoryAssetRepository()
export function assetFromJob(job: GenerationJob, mimeType = 'video/mp4'): Asset {
  return { id: crypto.randomUUID(), jobId: job.id, kind: job.kind === 'video' ? 'video' : 'image', mimeType, localPath: '', sizeBytes: 0, createdAt: new Date().toISOString() }
}
