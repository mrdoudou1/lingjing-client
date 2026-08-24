import type { ChatDelta, ChatRequest, GenerationJob, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'

const videoCapabilities: ModelCapabilities = {
  chat: { streaming: true, markdown: true },
  video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 },
}

export class MockGatewayAdapter implements GatewayAdapter {
  async testConnection() { return { ok: true, latencyMs: 42 } }
  async listModels() { return ['grok-imagine-video', 'veo-3', 'sora-2'] }
  async resolveCapabilities() { return videoCapabilities }
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
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> {
    return { id: crypto.randomUUID(), kind: 'video', status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
}
