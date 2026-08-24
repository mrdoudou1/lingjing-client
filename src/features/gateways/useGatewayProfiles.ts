import { useCallback, useEffect, useState } from 'react'
import type { GatewayProfile } from '../../types/domain'
import { gatewayProfileRepository } from '../../services/gateway/profileRepository'
import { gatewayRegistry } from '../../services/gateway/registry'

export function useGatewayProfiles() {
  const [profiles, setProfiles] = useState<GatewayProfile[]>([])
  const [testingId, setTestingId] = useState<string | null>(null)
  const [lastTest, setLastTest] = useState<Record<string, string>>({})
  useEffect(() => { void gatewayProfileRepository.list().then(setProfiles) }, [])
  const persist = useCallback((next: GatewayProfile[]) => { setProfiles(next); void gatewayProfileRepository.save(next) }, [])
  const setDefault = useCallback((id: string) => persist(profiles.map(profile => ({ ...profile, isDefault: profile.id === id }))), [persist, profiles])
  const testConnection = useCallback(async (profile: GatewayProfile) => { setTestingId(profile.id); const result = await gatewayRegistry.get('mock').testConnection(); setLastTest(previous => ({ ...previous, [profile.id]: result.ok ? `连接正常 · ${result.latencyMs}ms` : '连接失败' })); setTestingId(null) }, [])
  const addProfile = useCallback(() => { const id = `gateway-${Date.now()}`; const profile: GatewayProfile = { id, name: '新网关', baseUrl: 'https://example.com', protocol: 'openai-compatible', apiKeyRef: `system-keychain:${id}`, enabled: false, isDefault: false }; persist([...profiles, profile]) }, [persist, profiles])
  return { profiles, testingId, lastTest, setDefault, testConnection, addProfile }
}
