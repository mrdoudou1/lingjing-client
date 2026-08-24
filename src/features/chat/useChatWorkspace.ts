import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { ChatMessage, ChatSession } from '../../types/domain'
import { createId } from '../../lib/ids'
import { gatewayRegistry } from '../../services/gateway/registry'
import { ChatService } from '../../services/chat/chatService'
import { appPersistence } from '../../services/persistence/persistence'
import { tauriBridge } from '../../services/tauri/bridge'

const service = new ChatService(gatewayRegistry.get('mock'))
const initialSession = (): ChatSession => {
  const now = new Date().toISOString()
  return { id: createId('session'), title: '新的聊天', modelId: 'gpt-4.1', gatewayProfileId: 'mock-default', messages: [], createdAt: now, updatedAt: now }
}

export function useChatWorkspace() {
  const [sessions, setSessions] = useState<ChatSession[]>([])
  const [activeSessionId, setActiveSessionId] = useState('')
  const [models, setModels] = useState<string[]>(['gpt-4.1'])
  const [isStreaming, setIsStreaming] = useState(false)
  const controller = useRef<AbortController | null>(null)

  useEffect(() => {
    let mounted = true
    void Promise.all([appPersistence.get<ChatSession[]>('chat-sessions'), gatewayRegistry.get('mock').listModels()]).then(([saved, availableModels]) => {
      if (!mounted) return
      const restored = saved?.length ? saved : [initialSession()]
      setSessions(restored)
      setActiveSessionId(restored[0].id)
      setModels(['gpt-4.1', ...availableModels.filter(model => model !== 'gpt-4.1')])
    })
    return () => { mounted = false; controller.current?.abort() }
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void tauriBridge.listen<{ reason: string }>('app://shutdown', () => controller.current?.abort()).then(stop => { unlisten = stop }).catch(() => {})
    return () => { unlisten?.() }
  }, [])

  const activeSession = useMemo(() => sessions.find(session => session.id === activeSessionId) ?? sessions[0], [activeSessionId, sessions])
  const persist = useCallback((value: ChatSession[] | ((previous: ChatSession[]) => ChatSession[])) => {
    setSessions(previous => {
      const next = typeof value === 'function' ? value(previous) : value
      void appPersistence.set('chat-sessions', next)
      return next
    })
  }, [])

  const updateSession = useCallback((sessionId: string, updater: (session: ChatSession) => ChatSession) => {
    persist(previous => previous.map(session => session.id === sessionId ? updater(session) : session))
  }, [persist])

  const send = useCallback(async (content: string) => {
    if (!activeSession || !content.trim() || isStreaming) return
    const userMessage: ChatMessage = { id: createId('message'), role: 'user', content: content.trim(), status: 'completed', createdAt: new Date().toISOString() }
    const assistantMessage: ChatMessage = { id: createId('message'), role: 'assistant', content: '', status: 'streaming', createdAt: new Date().toISOString() }
    const sessionId = activeSession.id
    const nextSession: ChatSession = { ...activeSession, title: activeSession.messages.length ? activeSession.title : content.trim().slice(0, 24), messages: [...activeSession.messages, userMessage, assistantMessage], updatedAt: new Date().toISOString() }
    persist(previous => previous.map(session => session.id === sessionId ? nextSession : session))
    setIsStreaming(true)
    const abortController = new AbortController(); controller.current = abortController
    try {
      const request = { gatewayProfileId: nextSession.gatewayProfileId, modelId: nextSession.modelId, messages: nextSession.messages.filter(message => message.status !== 'streaming').map(({ role, content: messageContent }) => ({ role, content: messageContent })) }
      for await (const event of service.stream(request, abortController.signal)) {
        if (event.done) break
        updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, content: message.content + event.delta } : message), updatedAt: new Date().toISOString() }))
      }
      updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, status: abortController.signal.aborted ? 'stopped' : 'completed' } : message) }))
    } catch {
      updateSession(sessionId, session => ({ ...session, messages: session.messages.map(message => message.id === assistantMessage.id ? { ...message, status: 'failed', content: message.content || '生成失败，请重试。' } : message) }))
    } finally { controller.current = null; setIsStreaming(false) }
  }, [activeSession, isStreaming, persist, updateSession])

  const stop = useCallback(() => controller.current?.abort(), [])
  const createSession = useCallback(() => { const next = initialSession(); persist(previous => [next, ...previous]); setActiveSessionId(next.id) }, [persist])
  const deleteSession = useCallback((sessionId: string) => { const normalized = sessions.filter(session => session.id !== sessionId); const next = normalized.length ? normalized : [initialSession()]; persist(next); if (activeSessionId === sessionId) setActiveSessionId(next[0].id) }, [activeSessionId, persist, sessions])
  const setModel = useCallback((modelId: string) => { if (!activeSession) return; updateSession(activeSession.id, session => ({ ...session, modelId })) }, [activeSession, updateSession])

  return { sessions, activeSession, activeSessionId, setActiveSessionId, models, isStreaming, send, stop, createSession, deleteSession, setModel }
}
