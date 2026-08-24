import { useEffect, useState } from 'react'
import type { ModelCapabilities } from '../../types/domain'
import { gatewayRegistry } from '../../services/gateway/registry'

export function useModelCapabilities(modelId: string) {
  const [capabilities, setCapabilities] = useState<ModelCapabilities>({})
  const [loading, setLoading] = useState(true)
  useEffect(() => {
    let mounted = true
    void gatewayRegistry.runtime().resolveCapabilities(modelId).then(next => { if (mounted) { setCapabilities(next); setLoading(false) } }).catch(() => { if (mounted) { setCapabilities({}); setLoading(false) } })
    return () => { mounted = false }
  }, [modelId])
  return { capabilities, loading }
}
