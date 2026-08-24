import type { AudioRequest } from '../../types/domain'
import { assetRepository } from '../assets/assetRepository'
import type { GatewayAdapter } from '../gateway/types'
import type { JobProgress } from './jobManager'
import { historyRepository } from './historyRepository'
import { updateDesktopJob } from './desktopJobLifecycle'

export class AudioJobService {
  private readonly adapter: GatewayAdapter
  constructor(adapter: GatewayAdapter) { this.adapter = adapter }

  async start(request: AudioRequest, onProgress: (state: JobProgress) => void, onCreated?: (jobId: string) => void) {
    const job = await this.adapter.createAudioJob(request)
    onCreated?.(job.id)
    if (job.status === 'succeeded') {
      onProgress({ status: 'succeeded', progress: 100 })
      await assetRepository.reload()
      await historyRepository.recordJob(job, 'succeeded')
      return { ...job, status: 'succeeded' as const, progress: 100 }
    }
    updateDesktopJob(job.id, 'queued', 0)
    onProgress({ status: 'running', progress: 25 })
    updateDesktopJob(job.id, 'running', 25)
    await new Promise(resolve => window.setTimeout(resolve, 180))
    onProgress({ status: 'running', progress: 70 })
    updateDesktopJob(job.id, 'running', 70)
    await new Promise(resolve => window.setTimeout(resolve, 180))
    onProgress({ status: 'succeeded', progress: 100 })
    updateDesktopJob(job.id, 'succeeded', 100)
    await assetRepository.save({ id: `asset_${job.id}`, jobId: job.id, kind: 'audio', mimeType: request.kind === 'tts' ? 'audio/mpeg' : 'text/plain', localPath: `mock://assets/${job.id}.${request.format.toLowerCase()}`, sizeBytes: request.kind === 'tts' ? 128 * 1024 : 8 * 1024, createdAt: new Date().toISOString() })
    await historyRepository.recordJob(job, 'succeeded')
    return { ...job, status: 'succeeded' as const, progress: 100 }
  }
}
