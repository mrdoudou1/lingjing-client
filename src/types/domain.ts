export type Section = '聊天' | '图片' | '视频' | 'TTS / STT' | '素材库' | '图库' | '视频库' | '渠道' | '历史记录' | '设置'
export type Theme = 'system' | 'light' | 'dark'
export type VideoOperation = 'generate' | 'edit' | 'extend'
export type ImageResolution = '1k' | '2k' | string
export type JobStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'canceled' | 'stopped'

export type Notify = (message: string) => void

export type ChatRole = 'user' | 'assistant' | 'system'
export type ChatMessageStatus = 'streaming' | 'completed' | 'stopped' | 'failed'

export interface ChatMessage {
  id: string
  role: ChatRole
  content: string
  status: ChatMessageStatus
  createdAt: string
}

export interface ChatSession {
  id: string
  title: string
  modelId: string
  gatewayProfileId: string
  messages: ChatMessage[]
  createdAt: string
  updatedAt: string
}

export interface ChatRequest {
  gatewayProfileId: string
  modelId: string
  messages: Array<Pick<ChatMessage, 'role' | 'content'>>
}

export interface ChatDelta {
  delta: string
  done: boolean
}

export interface ImageRequest {
  gatewayProfileId: string
  modelId: string
  prompt: string
  count: number
  aspectRatio?: string
  resolution?: ImageResolution
  quality?: string
  referenceAssetIds?: string[]
}

export interface AudioRequest {
  gatewayProfileId: string
  modelId: string
  kind: 'tts' | 'stt'
  text?: string
  sourceFileName?: string
  voice?: string
  language?: string
  format: string
  speed?: number
}

export interface GatewayProfile {
  id: string
  name: string
  baseUrl: string
  protocol: 'openai-compatible' | 'newapi' | 'sub2api' | 'grok2api' | 'custom'
  apiKeyRef: string
  enabled: boolean
  isDefault: boolean
  createdAt?: string
  updatedAt?: string
}

export interface ModelCapabilities {
  chat?: { streaming: boolean; markdown: boolean }
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

export interface HistoryRecord {
  id: string
  jobId: string
  kind: 'chat' | 'image' | 'video' | 'tts' | 'stt'
  status: JobStatus
  modelId: string
  gatewayProfileId: string
  request: unknown
  createdAt: string
  finishedAt?: string
  errorMessage?: string
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
