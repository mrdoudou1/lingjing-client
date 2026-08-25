import { useMemo, useState } from 'react'
import type { Notify } from '../types/domain'
import { PageHeader } from '../components/common/PageHeader'
import { useHistory } from '../features/history/useHistory'

const labels = { queued: '排队中', running: '进行中', succeeded: '已完成', failed: '失败', canceled: '已取消', stopped: '已停止' } as const

export function VideoLibraryPage({ notify }: { notify: Notify }) {
  const history = useHistory()
  const [query, setQuery] = useState('')
  const records = useMemo(() => history.records.filter(record => record.kind === 'video' && (!query.trim() || `${record.jobId} ${record.modelId}`.toLowerCase().includes(query.trim().toLowerCase()))), [history.records, query])
  return <div className="video-library-page"><PageHeader eyebrow="任务归档" title="视频库" actions={<button className="light-button" onClick={() => { void history.refresh(); notify('视频库已刷新') }}>↻ 刷新</button>} /><div className="video-toolbar"><div className="search-box">⌕<input value={query} onChange={event => setQuery(event.target.value)} placeholder="搜索提示词或 ID" /></div><div className="job-stats"><span>☷ 任务总数 <strong>{records.length}</strong></span><span className="done">✓ 已完成 {records.filter(record => record.status === 'succeeded').length}</span><span className="failed">! 失败 {records.filter(record => record.status === 'failed').length}</span></div></div>{history.loading ? <div className="history-placeholder"><span>◌</span><h2>正在加载视频任务</h2><p>读取本地任务记录。</p></div> : records.length === 0 ? <div className="history-placeholder"><span>▣</span><h2>暂无视频任务</h2><p>完成视频生成或编辑后，任务会显示在这里。</p></div> : <div className="video-table"><div className="video-table-head"><span>状态</span><span>任务 ID</span><span>模型</span><span>状态 / 进度</span><span>网关</span><span>创建时间</span></div>{records.map(record => <button className="video-row" key={record.id} onClick={() => notify('已打开视频任务详情')}><span>□</span><span><strong>{record.jobId}</strong><small>{record.kind}</small></span><span>{record.modelId || '未指定模型'}</span><span className={record.status === 'failed' ? 'failed-dot' : record.status === 'succeeded' ? 'done' : 'running'}>{labels[record.status]}<small>{record.status === 'succeeded' ? '100%' : record.status === 'running' ? '进行中' : '—'}</small></span><span>{record.gatewayProfileId || '未指定网关'}</span><span>{new Date(record.createdAt).toLocaleString('zh-CN')}</span></button>)}</div>}</div>
}
