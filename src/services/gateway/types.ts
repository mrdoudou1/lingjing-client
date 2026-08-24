import type { ChatDelta, ChatRequest, GenerationJob, ModelCapabilities, VideoOperation } from '../../types/domain'

export interface VideoRequest {
  gatewayProfileId: string
  modelId: string
  operation: VideoOperation
  prompt: string
  durationSec?: number
  aspectRatio?: string
  resolution?: string
}

export interface GatewayAdapter {
  testConnection(): Promise<{ ok: boolean; latencyMs: number }>
  listModels(): Promise<string[]>
  resolveCapabilities(modelId: string): Promise<ModelCapabilities>
  chatStream(request: ChatRequest, signal?: AbortSignal): AsyncGenerator<ChatDelta>
  createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>>
}
