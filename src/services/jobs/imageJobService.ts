import type { ImageRequest } from '../../types/domain'
import { assetRepository } from '../assets/assetRepository'
import type { GatewayAdapter } from '../gateway/types'
import type { JobProgress } from './jobManager'

export class ImageJobService {
  private readonly adapter: GatewayAdapter

  constructor(adapter: GatewayAdapter) { this.adapter = adapter }

  async start(request: ImageRequest, onProgress: (state: JobProgress) => void, onCreated?: (jobId: string) => void) {
    const parent = await this.adapter.createImageJob(request)
    onCreated?.(parent.id)
    const total = Math.max(1, request.count)
    let completed = 0
    for (let index = 0; index < total; index += 1) {
      const child = await this.adapter.createImageJob({ ...request, count: 1 })
      await new Promise(resolve => window.setTimeout(resolve, 140))
      completed += 1
      onProgress({ status: completed === total ? 'succeeded' : 'running', progress: Math.round((completed / total) * 100) })
      await assetRepository.save({ id: `asset_${child.id}`, jobId: parent.id, kind: 'image', mimeType: 'image/png', localPath: `mock://assets/${child.id}.png`, thumbnailPath: `mock://assets/${child.id}.thumb.jpg`, sizeBytes: 256 * 1024, createdAt: new Date().toISOString() })
    }
    return { ...parent, status: 'succeeded' as const, progress: 100 }
  }
}
