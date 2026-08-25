import { useEffect, useState } from 'react'
import { gatewayRegistry } from '../../services/gateway/registry'
import { useDefaultGateway } from './useDefaultGateway'

export type GatewayModelKind = 'chat' | 'image' | 'video' | 'audio'

const fallbackModels: Record<GatewayModelKind, string[]> = {
  chat: ['gpt-4.1'],
  image: ['grok-imagine-image-2.0', 'flux-pro', 'gpt-image-1'],
  video: ['grok-imagine-video', 'veo-3', 'sora-2'],
  audio: ['mock-audio', 'tts-1', 'whisper-1'],
}

function matchesKind(model: string, kind: GatewayModelKind): boolean {
  const normalized = model.toLowerCase()
  if (kind === 'chat') return !matchesKind(model, 'image') && !matchesKind(model, 'video') && !matchesKind(model, 'audio')
  if (kind === 'image') return normalized.includes('image') || normalized.includes('flux')
  if (kind === 'video') return normalized.includes('video') || normalized.includes('veo') || normalized.includes('sora') || normalized.includes('kling') || normalized.includes('jimeng') || normalized.includes('hailuo') || normalized.includes('runway') || normalized.includes('vidu') || normalized.includes('wan')
  return normalized.includes('audio') || normalized.includes('speech') || normalized.includes('tts') || normalized.includes('whisper')
}

export function useGatewayModels(kind: GatewayModelKind) {
  const gatewayProfileId = useDefaultGateway()
  const [models, setModels] = useState(fallbackModels[kind])
  const [loading, setLoading] = useState(false)
  useEffect(() => {
    let mounted = true
    void gatewayRegistry.runtime().listModels(gatewayProfileId).then(all => {
      if (!mounted) return
      const filtered = all.filter(model => matchesKind(model, kind))
      setModels(filtered.length ? filtered : fallbackModels[kind])
      setLoading(false)
    }).catch(() => { if (mounted) { setModels(fallbackModels[kind]); setLoading(false) } })
    return () => { mounted = false }
  }, [gatewayProfileId, kind])
  return { gatewayProfileId, models, loading }
}
