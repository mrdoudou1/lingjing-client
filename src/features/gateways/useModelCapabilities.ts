import { useEffect, useState } from 'react'
import type { ModelCapabilities } from '../../types/domain'
import { gatewayRegistry } from '../../services/gateway/registry'
import { useDefaultGateway } from './useDefaultGateway'

export function useModelCapabilities(modelId: string) {
  const [capabilities, setCapabilities] = useState<ModelCapabilities>({})
  const [loading, setLoading] = useState(true)
  const gatewayProfileId = useDefaultGateway()
  useEffect(() => {
    let mounted = true
    void gatewayRegistry.runtime().resolveCapabilities(modelId, gatewayProfileId).then(next => { if (mounted) { setCapabilities(next); setLoading(false) } }).catch(() => { if (mounted) { setCapabilities({}); setLoading(false) } })
    return () => { mounted = false }
  }, [gatewayProfileId, modelId])
  return { capabilities, loading }
}
