import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { ChatMessage, ChatSession } from '../../types/domain'
import { createId } from '../../lib/ids'
import { gatewayRegistry } from '../../services/gateway/registry'
import { ChatService } from '../../services/chat/chatService'
import { appPersistence } from '../../services/persistence/persistence'
import { tauriBridge } from '../../services/tauri/bridge'
import { useGatewayModels } from '../gateways/useGatewayModels'

const service = new ChatService(gatewayRegistry.runtime())
const initialSession = (gatewayProfileId = 'mock-default'): ChatSession => {
  const now = new Date().toISOString()
  return { id: createId('session'), title: '新的聊天', modelId: 'gpt-4.1', gatewayProfileId, messages: [], createdAt: now, updatedAt: now }
}

export function useChatWorkspace() {
  const modelState = useGatewayModels('chat')
  const defaultGatewayId = modelState.gatewayProfileId
  const [sessions, setSessions] = useState<ChatSession[]>([])
  const [activeSessionId, setActiveSessionId] = useState('')
  const models = modelState.models
  const [isStreaming, setIsStreaming] = useState(false)
  const controller = useRef<AbortController | null>(null)

  useEffect(() => {
    let mounted = true
    const loadSessions = async () => {
      if (tauriBridge.available()) {
        try { return await tauriBridge.invoke<ChatSession[]>('chat_list_sessions') } catch { /* browser fallback */ }
      }
      return appPersistence.get<ChatSession[]>('chat-sessions')
    }
    void loadSessions().then(saved => {
      if (!mounted) return
      const restored = saved?.length ? saved : [initialSession(defaultGatewayId)]
      setSessions(restored)
      setActiveSessionId(restored[0].id)
    })
    return () => { mounted = false; controller.current?.abort() }
  }, [defaultGatewayId])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void tauriBridge.listen<{ reason: string }>('app://shutdown', () => controller.current?.abort()).then(stop => { unlisten = stop }).catch(() => {})
    return () => { unlisten?.() }
  }, [])

  const activeSession = useMemo(() => sessions.find(session => session.id === activeSessionId) ?? sessions[0], [activeSessionId, sessions])
  const persist = useCallback((value: ChatSession[] | ((previous: ChatSession[]) => ChatSession[])) => {
    setSessions(previous => {
      const next = typeof value === 'function' ? value(previous) : value
      if (tauriBridge.available()) {
        void Promise.all(next.map(session => tauriBridge.invoke('chat_save_session', { session }))).catch(() => {})
      } else {
        void appPersistence.set('chat-sessions', next)
      }
      return next
    })
  }, [])

  const updateSession = useCallback((sessionId: string, updater: (session: ChatSession) => ChatSession) => {
    persist(previous => previous.map(session => session.id === sessionId ? updater(session) : session))
  }, [persist])

  const send = useCallback(async (content: string, regenerate = false) => {
    if (!activeSession || !content.trim() || isStreaming) return
    const userMessage: ChatMessage = { id: createId('message'), role: 'user', content: content.trim(), status: 'completed', createdAt: new Date().toISOString() }
    const assistantMessage: ChatMessage = { id: createId('message'), role: 'assistant', content: '', status: 'streaming', createdAt: new Date().toISOString() }
    const sessionId = activeSession.id
    const lastUserIndex = regenerate ? activeSession.messages.map(message => message.role).lastIndexOf('user') : -1
    const baseMessages = lastUserIndex >= 0 ? activeSession.messages.slice(0, lastUserIndex + 1) : activeSession.messages
    const nextSession: ChatSession = { ...activeSession, title: activeSession.messages.length ? activeSession.title : content.trim().slice(0, 24), messages: [...(regenerate ? baseMessages : [...activeSession.messages, userMessage]), assistantMessage], updatedAt: new Date().toISOString() }
    persist(previous => previous.map(session => session.id === sessionId ? nextSession : session))
    setIsStreaming(true)
    const abortController = new AbortController(); controller.current = abortController
    try {
      const request = { sessionId, gatewayProfileId: nextSession.gatewayProfileId, modelId: nextSession.modelId, messages: nextSession.messages.filter(message => message.status !== 'streaming').map(({ role, content: messageContent }) => ({ role, content: messageContent })) }
      for await (const event of service.stream(request, abortController.signal)) {
        if (event.done) break
        updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, content: message.content + event.delta } : message), updatedAt: new Date().toISOString() }))
      }
      updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, status: abortController.signal.aborted ? 'stopped' : 'completed' } : message) }))
    } catch {
      updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, status: 'failed', content: message.content || '生成失败，请重试。' } : message) }))
    } finally { controller.current = null; setIsStreaming(false) }
  }, [activeSession, isStreaming, persist, updateSession])

  const regenerate = useCallback(() => {
    if (!activeSession || isStreaming) return
    const lastUser = [...activeSession.messages].reverse().find(message => message.role === 'user')
    if (lastUser) void send(lastUser.content, true)
  }, [activeSession, isStreaming, send])

  const stop = useCallback(() => controller.current?.abort(), [])
  const createSession = useCallback(() => { const next = initialSession(defaultGatewayId); persist(previous => [next, ...previous]); setActiveSessionId(next.id) }, [defaultGatewayId, persist])
  const deleteSession = useCallback((sessionId: string) => { const normalized = sessions.filter(session => session.id !== sessionId); const next = normalized.length ? normalized : [initialSession(defaultGatewayId)]; persist(next); if (tauriBridge.available()) void tauriBridge.invoke('chat_delete_session', { id: sessionId }).catch(() => {}); if (activeSessionId === sessionId) setActiveSessionId(next[0].id) }, [activeSessionId, defaultGatewayId, persist, sessions])
  const setModel = useCallback((modelId: string) => { if (!activeSession) return; updateSession(activeSession.id, session => ({ ...session, modelId })) }, [activeSession, updateSession])

  return { sessions, activeSession, activeSessionId, setActiveSessionId, models, isStreaming, send, regenerate, stop, createSession, deleteSession, setModel }
}
