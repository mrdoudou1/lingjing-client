import type { GatewayProfile } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'

const defaultProfile: GatewayProfile = {
  id: 'mock-default', name: 'Mock Gateway', baseUrl: 'mock://local', protocol: 'openai-compatible', apiKeyRef: 'system-keychain:mock-default', enabled: true, isDefault: true, createdAt: new Date(0).toISOString(), updatedAt: new Date(0).toISOString(),
}

export class GatewayProfileRepository {
  async list() { return (await appPersistence.get<GatewayProfile[]>('gateway-profiles')) ?? [defaultProfile] }
  async save(profiles: GatewayProfile[]) { const now = new Date().toISOString(); await appPersistence.set('gateway-profiles', profiles.map(profile => ({ ...profile, updatedAt: now, createdAt: profile.createdAt ?? now }))) }
}
export const gatewayProfileRepository = new GatewayProfileRepository()
