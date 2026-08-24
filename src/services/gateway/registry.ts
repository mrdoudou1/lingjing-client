import { MockGatewayAdapter } from './mockGateway'
import type { GatewayAdapter } from './types'
import { DesktopGatewayAdapter } from './desktopGateway'

export class GatewayRegistry {
  private readonly adapters = new Map<string, GatewayAdapter>([['mock', new MockGatewayAdapter()]])
  get(protocol: string): GatewayAdapter { return this.adapters.get(protocol) ?? this.adapters.get('mock')! }
  runtime(): GatewayAdapter { return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window ? new DesktopGatewayAdapter() : this.get('mock') }
}
export const gatewayRegistry = new GatewayRegistry()
