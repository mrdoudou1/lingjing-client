import type { AudioRequest, ChatDelta, ChatRequest, GenerationJob, ImageRequest, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'
import { tauriBridge } from '../tauri/bridge'

function capabilitiesFor(modelId: string): ModelCapabilities {
  const normalized = modelId.toLowerCase()
  if (normalized.includes('image') || normalized.includes('flux')) return { image: { count: { min: 1, max: 4, default: 1 }, aspectRatios: ['1 : 1', '16 : 9', '9 : 16'], resolutions: ['1k', '2k'], qualities: ['draft', 'standard', 'high'], supportsEdit: true } }
  if (normalized.includes('audio') || normalized.includes('speech') || normalized.includes('tts') || normalized.includes('whisper')) return { tts: { voices: ['Aria · 温暖女声', 'River · 平静男声'], formats: ['MP3', 'WAV'], streaming: false }, stt: { languages: ['中文（普通话）', 'English'], formats: ['TXT', 'JSON', 'SRT', 'VTT'], timestamps: true, realtime: false } }
  if (normalized.includes('video') || normalized.includes('veo') || normalized.includes('sora')) return { video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 } }
  return { chat: { streaming: true, markdown: true } }
}

export class DesktopGatewayAdapter implements GatewayAdapter {
  async testConnection(gatewayProfileId = 'mock-default') { return tauriBridge.invoke<{ ok: boolean; latencyMs: number }>('gateway_test_connection', { profileId: gatewayProfileId }) }
  async listModels(gatewayProfileId = 'mock-default') { return tauriBridge.invoke<string[]>('gateway_refresh_models', { profileId: gatewayProfileId }) }
  async resolveCapabilities(modelId: string, gatewayProfileId = 'mock-default') {
    try {
      return await tauriBridge.invoke<ModelCapabilities>('gateway_get_model_capabilities', { profileId: gatewayProfileId, modelId })
    } catch {
      return capabilitiesFor(modelId)
    }
  }
  async *chatStream(request: ChatRequest, signal?: AbortSignal): AsyncGenerator<ChatDelta> {
    const last = request.messages.at(-1)?.content ?? ''
    const sessionId = request.sessionId ?? 'desktop-session'
    type StreamEvent = { sessionId: string; delta?: string; correlationId: string; code?: string; message?: string }
    const queue: Array<ChatDelta | null> = []
    let wake: (() => void) | null = null
    let completed = false
    let streamError: Error | null = null
    const push = (event: ChatDelta | null) => { queue.push(event); wake?.(); wake = null }
    const unlistenDelta = await tauriBridge.listen<StreamEvent>('chat://delta', event => { if (event.sessionId === sessionId) push({ delta: event.delta ?? '', done: false }) })
    const unlistenCompleted = await tauriBridge.listen<StreamEvent>('chat://completed', event => { if (event.sessionId === sessionId) { completed = true; push(null) } })
    const unlistenError = await tauriBridge.listen<StreamEvent>('chat://error', event => { if (event.sessionId === sessionId) { streamError = new Error(event.message ?? event.code ?? 'Chat stream failed'); completed = true; push(null) } })
    const abort = () => { completed = true; push(null); void tauriBridge.invoke('chat_stop', { sessionId }).catch(() => {}) }
    signal?.addEventListener('abort', abort, { once: true })
    try {
      void tauriBridge.invoke('chat_stream', { input: { gateway_profile_id: request.gatewayProfileId, model_id: request.modelId, session_id: sessionId, content: last, messages: request.messages } }).catch(error => { streamError = error instanceof Error ? error : new Error(String(error)); completed = true; push(null) })
      while (!completed || queue.length) {
        if (signal?.aborted) return
        if (!queue.length) await new Promise<void>(resolve => { wake = resolve })
        const event = queue.shift()
        if (event) yield event
      }
      if (streamError && !signal?.aborted) throw streamError
      yield { delta: '', done: true }
    } finally { signal?.removeEventListener('abort', abort); unlistenDelta(); unlistenCompleted(); unlistenError() }
  }
  async createImageJob(request: ImageRequest): Promise<GenerationJob<ImageRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>('image_create_job', { request }), request, 'image') }
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>('video_create_job', { request }), request, 'video') }
  async createAudioJob(request: AudioRequest): Promise<GenerationJob<AudioRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>(request.kind === 'tts' ? 'audio_tts' : 'audio_stt', { request }), request, request.kind) }
  private mapJob<T>(raw: Record<string, unknown>, request: T, kind: GenerationJob<T>['kind']): GenerationJob<T> {
    const status = String(raw.status ?? 'queued') as GenerationJob<T>['status']
    return { id: String(raw.id ?? crypto.randomUUID()), kind, status, progress: Number(raw.progress ?? 0), request, createdAt: String(raw.created_at ?? new Date().toISOString()) }
  }
}
