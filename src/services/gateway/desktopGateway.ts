import type { AudioRequest, ChatDelta, ChatRequest, GenerationJob, ImageRequest, ModelCapabilities } from '../../types/domain'
import type { GatewayAdapter, VideoRequest } from './types'
import { tauriBridge } from '../tauri/bridge'

function capabilitiesFor(modelId: string): ModelCapabilities {
  if (modelId.includes('image') || modelId.includes('flux')) return { image: { count: { min: 1, max: 4, default: 1 }, aspectRatios: ['1 : 1', '16 : 9', '9 : 16'], resolutions: ['1k', '2k'], qualities: ['draft', 'standard', 'high'], supportsEdit: true } }
  if (modelId.includes('audio') || modelId.includes('speech')) return { tts: { voices: ['Aria · 温暖女声', 'River · 平静男声'], formats: ['MP3', 'WAV'], streaming: false }, stt: { languages: ['中文（普通话）', 'English'], formats: ['TXT', 'JSON', 'SRT', 'VTT'], timestamps: true, realtime: false } }
  return { video: { operations: ['generate', 'edit', 'extend'], durations: [6, 12, 18], aspectRatios: ['16:9', '9:16', '1:1'], resolutions: ['720p', '1080p'], maxReferenceImages: 3, maxReferenceVoices: 1 } }
}

export class DesktopGatewayAdapter implements GatewayAdapter {
  async testConnection() { return tauriBridge.invoke<{ ok: boolean; latencyMs: number }>('gateway_test_connection', { profileId: 'mock-default' }) }
  async listModels() { return tauriBridge.invoke<string[]>('gateway_refresh_models', { profileId: 'mock-default' }) }
  async resolveCapabilities(modelId: string) { return capabilitiesFor(modelId) }
  async *chatStream(request: ChatRequest, signal?: AbortSignal): AsyncGenerator<ChatDelta> {
    const last = request.messages.at(-1)?.content ?? ''
    type StreamEvent = { sessionId: string; delta?: string; correlationId: string }
    const queue: Array<ChatDelta | null> = []
    let wake: (() => void) | null = null
    let completed = false
    const push = (event: ChatDelta | null) => { queue.push(event); wake?.(); wake = null }
    const unlistenDelta = await tauriBridge.listen<StreamEvent>('chat://delta', event => push({ delta: event.delta ?? '', done: false }))
    const unlistenCompleted = await tauriBridge.listen<StreamEvent>('chat://completed', () => { completed = true; push(null) })
    try {
      void tauriBridge.invoke('chat_stream', { input: { gateway_profile_id: request.gatewayProfileId, model_id: request.modelId, session_id: 'desktop-session', content: last } })
      while (!completed || queue.length) {
        if (signal?.aborted) return
        if (!queue.length) await new Promise<void>(resolve => { wake = resolve })
        const event = queue.shift()
        if (event) yield event
      }
      yield { delta: '', done: true }
    } finally { unlistenDelta(); unlistenCompleted() }
  }
  async createImageJob(request: ImageRequest): Promise<GenerationJob<ImageRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>('image_create_job', { request }), request, 'image') }
  async createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>('video_create_job', { request }), request, 'video') }
  async createAudioJob(request: AudioRequest): Promise<GenerationJob<AudioRequest>> { return this.mapJob(await tauriBridge.invoke<Record<string, unknown>>(request.kind === 'tts' ? 'audio_tts' : 'audio_stt', { request }), request, request.kind) }
  private mapJob<T>(raw: Record<string, unknown>, request: T, kind: GenerationJob<T>['kind']): GenerationJob<T> { return { id: String(raw.id ?? crypto.randomUUID()), kind, status: 'queued', progress: Number(raw.progress ?? 0), request, createdAt: String(raw.created_at ?? new Date().toISOString()) } }
}
