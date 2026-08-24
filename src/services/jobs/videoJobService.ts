import { assetRepository } from '../assets/assetRepository'
import type { GatewayAdapter, VideoRequest } from '../gateway/types'
import { MockJobManager, type JobProgress } from './jobManager'
import { historyRepository } from './historyRepository'

export class VideoJobService {
  private readonly manager = new MockJobManager()
  private readonly controllers = new Map<string, AbortController>()
  private readonly requests = new Map<string, VideoRequest>()
  private readonly adapter: GatewayAdapter

  constructor(adapter: GatewayAdapter) { this.adapter = adapter }

  async start(request: VideoRequest, onProgress: (state: JobProgress) => void, onCreated?: (jobId: string) => void) {
    const job = await this.adapter.createVideoJob(request)
    onCreated?.(job.id)
    const controller = new AbortController()
    this.controllers.set(job.id, controller)
    this.requests.set(job.id, request)
    try {
      const result = await this.manager.runVideoJob(job, onProgress, controller.signal)
      await historyRepository.recordJob(job, result.status)
      if (result.status === 'succeeded') await assetRepository.save({ id: `asset_${job.id}`, jobId: job.id, kind: 'video', mimeType: 'video/mp4', localPath: `mock://assets/${job.id}.mp4`, thumbnailPath: `mock://assets/${job.id}.jpg`, sizeBytes: 1024 * 1024, createdAt: new Date().toISOString() })
      return result
    } finally { this.controllers.delete(job.id) }
  }

  cancel(jobId: string) { this.controllers.get(jobId)?.abort() }

  async retry(jobId: string, onProgress: (state: JobProgress) => void, onCreated?: (newJobId: string) => void) {
    const request = this.requests.get(jobId)
    if (!request) throw new Error('视频任务不存在，无法重试')
    return this.start(request, onProgress, onCreated)
  }
}
