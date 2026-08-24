import type { AudioRequest, ChatDelta, ChatRequest, GenerationJob, ImageRequest, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'

const imageCapabilities: ModelCapabilities = {
  chat: { streaming: true, markdown: true },
  image: { count: { min: 1, max: 4, default: 1 }, aspectRatios: ['1 : 1', '16 : 9', '9 : 16'], resolutions: ['1k', '2k'], qualities: ['draft', 'standard', 'high'], supportsEdit: true },
}
const videoCapabilities: ModelCapabilities = {
  chat: { streaming: true, markdown: true },
  video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 },
}
const audioCapabilities: ModelCapabilities = {
  tts: { voices: ['Aria · 温暖女声', 'River · 平静男声'], formats: ['MP3', 'WAV'], streaming: false },
  stt: { languages: ['中文（普通话）', 'English'], formats: ['TXT', 'JSON', 'SRT', 'VTT'], timestamps: true, realtime: false },
}

export class MockGatewayAdapter implements GatewayAdapter {
  async testConnection(_gatewayProfileId?: string) { return { ok: true, latencyMs: 42 } }
  async listModels(_gatewayProfileId?: string) { return ['gpt-4.1', 'grok-imagine-image-2.0', 'flux-pro', 'gpt-image-1', 'grok-imagine-video', 'veo-3', 'sora-2'] }
  async resolveCapabilities(modelId: string, _gatewayProfileId?: string) {
    const normalized = modelId.toLowerCase()
    if (normalized.includes('image') || normalized.includes('flux')) return imageCapabilities
    if (normalized.includes('audio') || normalized.includes('speech') || normalized.includes('tts') || normalized.includes('whisper')) return audioCapabilities
    if (normalized.includes('video') || normalized.includes('veo') || normalized.includes('sora')) return videoCapabilities
    return { chat: { streaming: true, markdown: true } }
  }
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
  async createAudioJob(request: AudioRequest): Promise<GenerationJob<AudioRequest>> {
    return { id: crypto.randomUUID(), kind: request.kind, status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> {
    return { id: crypto.randomUUID(), kind: 'video', status: 'queued', progress: 0, request, createdAt: new Date().toISOString() }
  }
}
