export type Section = '聊天' | '图片' | '视频' | 'TTS / STT' | '图库' | '视频库' | '渠道' | '历史记录' | '设置'
export type Theme = 'system' | 'light' | 'dark'
export type VideoOperation = 'generate' | 'edit' | 'extend'
export type JobStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'canceled' | 'stopped'

export type Notify = (message: string) => void

export interface GatewayProfile {
  id: string
  name: string
  baseUrl: string
  protocol: 'openai-compatible' | 'newapi' | 'sub2api' | 'grok2api' | 'custom'
  apiKeyRef: string
  enabled: boolean
  isDefault: boolean
}

export interface ModelCapabilities {
  image?: { count?: { min: number; max: number; default: number }; aspectRatios: string[]; resolutions: string[]; qualities: string[]; supportsEdit: boolean }
  video?: { operations: VideoOperation[]; durations?: number[]; aspectRatios: string[]; resolutions: string[]; maxReferenceImages?: number; maxReferenceVoices?: number }
  tts?: { voices: string[]; formats: string[]; streaming: boolean }
  stt?: { languages: string[]; formats: string[]; timestamps: boolean; realtime: boolean }
}

export interface GenerationJob<TRequest = unknown> {
  id: string
  kind: 'chat' | 'image' | 'video' | 'tts' | 'stt'
  status: JobStatus
  progress: number
  request: TRequest
  errorMessage?: string
  createdAt: string
}

export interface Asset {
  id: string
  jobId?: string
  kind: 'image' | 'video' | 'audio'
  mimeType: string
  localPath: string
  thumbnailPath?: string
  sizeBytes: number
  createdAt: string
  favorite?: boolean
}
