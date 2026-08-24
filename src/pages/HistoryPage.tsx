import type { Notify, HistoryRecord } from '../types/domain'
import { useHistory } from '../features/history/useHistory'

const labels: Record<HistoryRecord['kind'], string> = { chat: '聊天', image: '图片', video: '视频', tts: 'TTS', stt: 'STT' }
export function HistoryPage({ notify }: { notify: Notify }) {
  const history = useHistory()
  return <div className="library-page"><div className="page-heading"><div><span className="eyebrow">活动记录</span><h1>历史记录</h1></div><button className="light-button" onClick={() => { void history.refresh(); notify('历史记录已刷新') }}>↻ 刷新</button></div><div className="filter-row">{(['全部','chat','image','video','tts','stt'] as const).map(item => <button key={item} className={history.filter === item ? 'selected' : ''} onClick={() => history.setFilter(item === '全部' ? '全部' : item)}>{item === '全部' ? '全部' : labels[item]}</button>)}</div>{history.loading ? <div className="history-placeholder"><span>◌</span><h2>正在加载历史</h2><p>读取本地任务记录。</p></div> : history.visible.length === 0 ? <div className="history-placeholder"><span>◷</span><h2>暂无历史记录</h2><p>完成聊天、图片、视频或音频任务后会显示在这里。</p></div> : <div className="history-list">{history.visible.map(record => <button className="history-row" key={record.id} onClick={() => notify(`已打开 ${labels[record.kind]} 任务详情`)}><span className={`history-status ${record.status}`}>{record.status}</span><span><strong>{labels[record.kind]}任务</strong><small>{record.modelId || '未指定模型'} · {record.jobId.slice(0, 12)}</small></span><span><small>{new Date(record.createdAt).toLocaleString('zh-CN')}</small></span></button>)}</div>}</div>
}
