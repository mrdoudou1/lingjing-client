import type { ChatDelta, ChatRequest, GenerationJob, ImageRequest, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'

const imageCapabilities: ModelCapabilities = {
  chat: { streaming: true, markdown: true },
  image: { count: { min: 1, max: 4, default: 1 }, aspectRatios: ['1 : 1', '16 : 9', '9 : 16'], resolutions: ['1k', '2k'], qualities: ['draft', 'standard', 'high'], supportsEdit: true },
}
const videoCapabilities: ModelCapabilities = {
  chat: { streaming: true, markdown: true },
  video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 },
}

export class MockGatewayAdapter implements GatewayAdapter {
  async testConnection() { return { ok: true, latencyMs: 42 } }
  async listModels() { return ['gpt-4.1', 'grok-imagine-image-2.0', 'flux-pro', 'gpt-image-1', 'grok-imagine-video', 'veo-3', 'sora-2'] }
  async resolveCapabilities(modelId: string) { return modelId.includes('image') || modelId.includes('flux') ? imageCapabilities : videoCapabilities }
  async *chatStream(request: ChatRequest, signal?: AbortSignal): AsyncGenerator<ChatDelta> {
    const last = request.messages.at(-1)?.content ?? ''
    const response = `已收到你的请求：**${last}**\n\n这是 Mock Gateway 的流式响应。接入真实网关时，只需要替换 GatewayAdapter，不需要改动聊天页面。`
    for (const chunk of response.match(/.{1,3}/gs) ?? []) {
      if (signal?.aborted) return
      await new Promise((resolve) => window.setTimeout(resolve, 45))
      yield { delta: chunk, done: false }
    }
    yield { delta: '', done: true }
  }
  async createImageJob(request: ImageRequest): Promise<GenerationJob<ImageRequest>> {
    return { id: crypto.randomUUID(), kind: 'image', status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> {
    return { id: crypto.randomUUID(), kind: 'video', status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
}
