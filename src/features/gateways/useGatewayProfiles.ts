import { useCallback, useEffect, useState } from 'react'
import type { GatewayProfile } from '../../types/domain'
import { gatewayProfileRepository } from '../../services/gateway/profileRepository'
import { gatewayRegistry } from '../../services/gateway/registry'
import { tauriBridge } from '../../services/tauri/bridge'

type RustGatewayProfile = {
  id: string
  name: string
  base_url: string
  protocol: GatewayProfile['protocol']
  api_key_ref: string
  enabled: boolean
  is_default: boolean
  created_at?: string
  updated_at?: string
}

const fromRustProfile = (profile: RustGatewayProfile): GatewayProfile => ({
  id: profile.id,
  name: profile.name,
  baseUrl: profile.base_url,
  protocol: profile.protocol,
  apiKeyRef: profile.api_key_ref,
  enabled: profile.enabled,
  isDefault: profile.is_default,
  createdAt: profile.created_at,
  updatedAt: profile.updated_at,
})

const toRustProfile = (profile: GatewayProfile): RustGatewayProfile => ({
  id: profile.id,
  name: profile.name,
  base_url: profile.baseUrl,
  protocol: profile.protocol,
  api_key_ref: profile.apiKeyRef,
  enabled: profile.enabled,
  is_default: profile.isDefault,
  created_at: profile.createdAt,
  updated_at: profile.updatedAt,
})

export function useGatewayProfiles() {
  const [profiles, setProfiles] = useState<GatewayProfile[]>([])
  const [testingId, setTestingId] = useState<string | null>(null)
  const [lastTest, setLastTest] = useState<Record<string, string>>({})
  useEffect(() => {
    const load = async () => {
      if (tauriBridge.available()) {
        try {
          const stored = await tauriBridge.invoke<RustGatewayProfile[]>('gateway_list_profiles')
          setProfiles(stored.map(fromRustProfile))
          return
        } catch { /* fall back to browser storage during desktop bootstrap */ }
      }
      setProfiles(await gatewayProfileRepository.list())
    }
    void load()
  }, [])
  const persist = useCallback((next: GatewayProfile[]) => {
    const now = new Date().toISOString()
    const stamped = next.map(profile => ({ ...profile, createdAt: profile.createdAt ?? now, updatedAt: now }))
    setProfiles(stamped)
    if (!tauriBridge.available()) void gatewayProfileRepository.save(stamped)
  }, [])
  const updateProfile = useCallback((id: string, patch: Partial<GatewayProfile>) => {
    const next = profiles.map(profile => profile.id === id ? { ...profile, ...patch } : profile)
    persist(next)
    const updated = next.find(profile => profile.id === id)
    if (updated && tauriBridge.available()) void tauriBridge.invoke('gateway_update_profile', { profile: toRustProfile(updated) })
  }, [persist, profiles])
  const setDefault = useCallback(async (id: string) => { const next = profiles.map(profile => ({ ...profile, isDefault: profile.id === id })); persist(next); if (tauriBridge.available()) await tauriBridge.invoke('gateway_set_default', { id }) }, [persist, profiles])
  const testConnection = useCallback(async (profile: GatewayProfile) => { setTestingId(profile.id); try { const result = await gatewayRegistry.runtime().testConnection(profile.id); setLastTest(previous => ({ ...previous, [profile.id]: result.ok ? `连接正常 · ${result.latencyMs}ms` : '连接失败' })) } catch (error) { setLastTest(previous => ({ ...previous, [profile.id]: `连接失败 · ${error instanceof Error ? error.message : '未知错误'}` })) } finally { setTestingId(null) } }, [])
  const refreshModels = useCallback(async (profile: GatewayProfile) => { const models = await gatewayRegistry.runtime().listModels(profile.id); setLastTest(previous => ({ ...previous, [profile.id]: `模型已刷新 · ${models.length} 个` })); return models }, [])
  const setApiKey = useCallback(async (profile: GatewayProfile, secret: string) => { if (!secret.trim()) throw new Error('API Key 不能为空'); if (tauriBridge.available()) { const reference = await tauriBridge.invoke<string>('gateway_set_api_key', { profileId: profile.id, secret }); updateProfile(profile.id, { apiKeyRef: reference, enabled: true }) } else { updateProfile(profile.id, { apiKeyRef: `system-keychain:${profile.id}`, enabled: true }) } }, [updateProfile])
  const clearApiKey = useCallback(async (profile: GatewayProfile) => {
    if (tauriBridge.available()) await tauriBridge.invoke('gateway_clear_api_key', { profileId: profile.id })
    updateProfile(profile.id, { enabled: false })
  }, [updateProfile])
  const addProfile = useCallback(() => {
    const id = 'gateway-' + Date.now()
    const profile: GatewayProfile = { id, name: '新网关', baseUrl: 'https://example.com', protocol: 'openai-compatible', apiKeyRef: 'system-keychain:' + id, enabled: false, isDefault: profiles.length === 0 }
    persist([...profiles, profile])
    if (tauriBridge.available()) void tauriBridge.invoke('gateway_create_profile', { profile: toRustProfile(profile) })
  }, [persist, profiles])
  const deleteProfile = useCallback(async (profile: GatewayProfile) => {
    const next = profiles.filter(item => item.id !== profile.id)
    persist(next)
    if (tauriBridge.available()) await tauriBridge.invoke('gateway_delete_profile', { id: profile.id })
  }, [persist, profiles])
  return { profiles, testingId, lastTest, setDefault, updateProfile, testConnection, refreshModels, setApiKey, clearApiKey, addProfile, deleteProfile }
}
