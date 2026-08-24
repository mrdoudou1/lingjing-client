import type { ChatDelta, ChatRequest, GenerationJob, ModelCapabilities, VideoOperation } from '../../types/domain'

export interface VideoRequest {
  gatewayProfileId: string
  modelId: string
  operation: VideoOperation
  inputMode?: 'text-to-video' | 'image-to-video' | 'reference-to-video'
  prompt: string
  firstFrameAssetId?: string
  referenceImageAssetIds?: string[]
  referenceVoiceIds?: string[]
  sourceVideoAssetId?: string
  durationSec?: number
  extensionDurationSec?: number
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
