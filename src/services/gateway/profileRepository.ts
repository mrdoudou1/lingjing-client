import type { GatewayProfile } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'

const defaultProfile: GatewayProfile = {
  id: 'mock-default', name: 'Mock Gateway', baseUrl: 'mock://local', protocol: 'openai-compatible', apiKeyRef: 'system-keychain:mock-default', enabled: true, isDefault: true,
}

export class GatewayProfileRepository {
  async list() { return (await appPersistence.get<GatewayProfile[]>('gateway-profiles')) ?? [defaultProfile] }
  async save(profiles: GatewayProfile[]) { await appPersistence.set('gateway-profiles', profiles) }
}
export const gatewayProfileRepository = new GatewayProfileRepository()
