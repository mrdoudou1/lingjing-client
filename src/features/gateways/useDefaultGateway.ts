import { useEffect, useState } from 'react'
import type { GatewayProfile } from '../../types/domain'
import { gatewayProfileRepository } from '../../services/gateway/profileRepository'
import { tauriBridge } from '../../services/tauri/bridge'

type RustGatewayProfile = { id: string; is_default: boolean; enabled: boolean }

export function useDefaultGateway() {
  const [gatewayProfileId, setGatewayProfileId] = useState('mock-default')
  useEffect(() => {
    let mounted = true
    const load = async () => {
      try {
        const profiles = tauriBridge.available()
          ? await tauriBridge.invoke<RustGatewayProfile[]>('gateway_list_profiles')
          : await gatewayProfileRepository.list() as GatewayProfile[]
        const selected = profiles.find(profile => profile.enabled && ('is_default' in profile ? profile.is_default : profile.isDefault))
          ?? profiles.find(profile => profile.enabled)
        if (mounted && selected) setGatewayProfileId(selected.id)
      } catch {
        // Keep the local mock profile while the desktop bridge is booting.
      }
    }
    void load()
    return () => { mounted = false }
  }, [])
  return gatewayProfileId
}
