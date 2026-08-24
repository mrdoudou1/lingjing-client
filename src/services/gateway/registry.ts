import { MockGatewayAdapter } from './mockGateway'
import type { GatewayAdapter } from './types'

export class GatewayRegistry {
  private readonly adapters = new Map<string, GatewayAdapter>([['mock', new MockGatewayAdapter()]])
  get(protocol: string): GatewayAdapter { return this.adapters.get(protocol) ?? this.adapters.get('mock')! }
}
export const gatewayRegistry = new GatewayRegistry()
