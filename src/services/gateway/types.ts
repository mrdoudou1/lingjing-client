import type { AudioRequest, ChatDelta, ChatRequest, GenerationJob, ImageRequest, ModelCapabilities, VideoOperation } from '../../types/domain'

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
  testConnection(gatewayProfileId?: string): Promise<{ ok: boolean; latencyMs: number }>
  listModels(gatewayProfileId?: string): Promise<string[]>
  resolveCapabilities(modelId: string, gatewayProfileId?: string): Promise<ModelCapabilities>
  chatStream(request: ChatRequest, signal?: AbortSignal): AsyncGenerator<ChatDelta>
  createImageJob(request: ImageRequest): Promise<GenerationJob<ImageRequest>>
  createAudioJob(request: AudioRequest): Promise<GenerationJob<AudioRequest>>
  createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>>
}
