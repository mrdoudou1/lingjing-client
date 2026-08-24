import type { ChatRequest } from '../../types/domain'
import type { GatewayAdapter } from '../gateway/types'

export class ChatService {
  private readonly adapter: GatewayAdapter

  constructor(adapter: GatewayAdapter) {
    this.adapter = adapter
  }

  stream(request: ChatRequest, signal?: AbortSignal) {
    return this.adapter.chatStream(request, signal)
  }
}
