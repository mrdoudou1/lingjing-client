import { useCallback, useEffect, useState } from 'react'
import type { GatewayProfile } from '../../types/domain'
import { gatewayProfileRepository } from '../../services/gateway/profileRepository'
import { gatewayRegistry } from '../../services/gateway/registry'
import { tauriBridge } from '../../services/tauri/bridge'

export function useGatewayProfiles() {
  const [profiles, setProfiles] = useState<GatewayProfile[]>([])
  const [testingId, setTestingId] = useState<string | null>(null)
  const [lastTest, setLastTest] = useState<Record<string, string>>({})
  useEffect(() => { void gatewayProfileRepository.list().then(setProfiles) }, [])
  const persist = useCallback((next: GatewayProfile[]) => { setProfiles(next); void gatewayProfileRepository.save(next) }, [])
  const updateProfile = useCallback((id: string, patch: Partial<GatewayProfile>) => { persist(profiles.map(profile => profile.id === id ? { ...profile, ...patch } : profile)) }, [persist, profiles])
  const setDefault = useCallback(async (id: string) => { const next = profiles.map(profile => ({ ...profile, isDefault: profile.id === id })); persist(next); if (tauriBridge.available()) await tauriBridge.invoke('gateway_set_default', { id }) }, [persist, profiles])
  const testConnection = useCallback(async (profile: GatewayProfile) => { setTestingId(profile.id); try { const result = await gatewayRegistry.runtime().testConnection(); setLastTest(previous => ({ ...previous, [profile.id]: result.ok ? `连接正常 · ${result.latencyMs}ms` : '连接失败' })) } catch (error) { setLastTest(previous => ({ ...previous, [profile.id]: `连接失败 · ${error instanceof Error ? error.message : '未知错误'}` })) } finally { setTestingId(null) } }, [])
  const refreshModels = useCallback(async (profile: GatewayProfile) => { const models = await gatewayRegistry.runtime().listModels(); setLastTest(previous => ({ ...previous, [profile.id]: `模型已刷新 · ${models.length} 个` })); return models }, [])
  const setApiKey = useCallback(async (profile: GatewayProfile, secret: string) => { if (!secret.trim()) throw new Error('API Key 不能为空'); if (tauriBridge.available()) { const reference = await tauriBridge.invoke<string>('gateway_set_api_key', { profileId: profile.id, secret }); updateProfile(profile.id, { apiKeyRef: reference, enabled: true }) } else { updateProfile(profile.id, { apiKeyRef: `system-keychain:${profile.id}`, enabled: true }) } }, [updateProfile])
  const addProfile = useCallback(() => { const id = `gateway-${Date.now()}`; const profile: GatewayProfile = { id, name: '新网关', baseUrl: 'https://example.com', protocol: 'openai-compatible', apiKeyRef: `system-keychain:${id}`, enabled: false, isDefault: profiles.length === 0 }; persist([...profiles, profile]) }, [persist, profiles])
  const deleteProfile = useCallback(async (profile: GatewayProfile) => { const next = profiles.filter(item => item.id !== profile.id); persist(next); if (tauriBridge.available()) await tauriBridge.invoke('gateway_delete_profile', { id: profile.id }) }, [persist, profiles])
  return { profiles, testingId, lastTest, setDefault, updateProfile, testConnection, refreshModels, setApiKey, addProfile, deleteProfile }
}
