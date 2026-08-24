import { useState } from 'react'
import type { Notify } from '../types/domain'

export function ChatPage({ notify }: { notify: Notify }) {
  const [prompt, setPrompt] = useState('')
  const [model, setModel] = useState('gpt-4.1')
  const [sent, setSent] = useState(false)
  const send = () => { if (!prompt.trim()) return notify('请输入消息'); setSent(true); setPrompt(''); notify('消息已发送') }
  return <div className="creative-page chat-page"><div className="creative-header"><div><span className="eyebrow">聊天工作区</span><h1>聊天</h1></div></div><div className="chat-canvas"><div className="empty-orb">✦</div><h2>{sent ? '正在为你整理答案' : '今天想做什么？'}</h2><p>{sent ? '生成结果会出现在这里，你可以继续追问。' : '用一句话开始一段新的创作。'}</p></div><div className="chat-composer"><textarea className="unified-input" autoFocus value={prompt} onChange={e => setPrompt(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send() } }} placeholder="输入消息，描述你的想法…" rows={4}/><div className="composer-footer"><div className="composer-controls"><button onClick={() => notify('附件选择器已打开')}>＋</button><button onClick={() => notify('已开启深度思考')}>✧ 深度思考</button><select value={model} onChange={e => setModel(e.target.value)}><option>gpt-4.1</option><option>claude-4-sonnet</option><option>gemini-2.5-pro</option></select></div><button className="send-button" onClick={send}>↑</button></div></div></div>
}
