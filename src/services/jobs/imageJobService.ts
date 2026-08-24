import type { ImageRequest } from '../../types/domain'
import { assetRepository } from '../assets/assetRepository'
import type { GatewayAdapter } from '../gateway/types'
import type { JobProgress } from './jobManager'
import { historyRepository } from './historyRepository'
import { updateDesktopJob } from './desktopJobLifecycle'

export class ImageJobService {
  private readonly adapter: GatewayAdapter

  constructor(adapter: GatewayAdapter) { this.adapter = adapter }

  async start(request: ImageRequest, onProgress: (state: JobProgress) => void, onCreated?: (jobId: string) => void) {
    const parent = await this.adapter.createImageJob(request)
    onCreated?.(parent.id)
    updateDesktopJob(parent.id, 'queued', 0)
    const total = Math.max(1, request.count)
    let completed = 0
    for (let index = 0; index < total; index += 1) {
      const child = await this.adapter.createImageJob({ ...request, count: 1 })
      await new Promise(resolve => window.setTimeout(resolve, 140))
      completed += 1
      const progress = Math.round((completed / total) * 100)
      const status = completed === total ? 'succeeded' : 'running'
      onProgress({ status, progress })
      updateDesktopJob(parent.id, status, progress)
      await assetRepository.save({ id: `asset_${child.id}`, jobId: parent.id, kind: 'image', mimeType: 'image/png', localPath: `mock://assets/${child.id}.png`, thumbnailPath: `mock://assets/${child.id}.thumb.jpg`, sizeBytes: 256 * 1024, createdAt: new Date().toISOString() })
    }
    await historyRepository.recordJob(parent, 'succeeded')
    return { ...parent, status: 'succeeded' as const, progress: 100 }
  }
}
