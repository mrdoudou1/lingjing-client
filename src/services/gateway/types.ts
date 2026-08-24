import type { GenerationJob, ModelCapabilities, VideoOperation } from '../../types/domain'

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
  listModels(): Promise<string[]>
  resolveCapabilities(modelId: string): Promise<ModelCapabilities>
  createVideoJob(request: VideoRequest): Promise<GenerationJob<VideoRequest>>
}
