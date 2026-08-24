import type { GenerationJob, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'

const videoCapabilities: ModelCapabilities = {
  video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 },
}

export class MockGatewayAdapter implements GatewayAdapter {
  async listModels() { return ['grok-imagine-video', 'veo-3', 'sora-2'] }
  async resolveCapabilities() { return videoCapabilities }
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> {
    return { id: crypto.randomUUID(), kind: 'video', status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
}
