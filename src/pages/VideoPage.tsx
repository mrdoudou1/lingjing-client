import { useState } from 'react'
import type { Notify, VideoOperation } from '../types/domain'
import { gatewayRegistry } from '../services/gateway/registry'
import { jobManager } from '../services/jobs/jobManager'
import type { VideoRequest } from '../services/gateway/types'

export function VideoPage({ notify }: { notify: Notify }) {
  const [operation, setOperation] = useState<VideoOperation>('generate')
  const [prompt, setPrompt] = useState('')
  const [model, setModel] = useState('grok-imagine-video')
  const [durationSec, setDurationSec] = useState(6)
  const [aspectRatio, setAspectRatio] = useState('16:9')
  const [resolution, setResolution] = useState('720p')
  const [sourceAssetId, setSourceAssetId] = useState('')
  const [progress, setProgress] = useState(0)
  const [status, setStatus] = useState<'idle' | 'running' | 'succeeded' | 'failed'>('idle')

  const createJob = async () => {
    if (!prompt.trim()) return notify('请输入视频描述')
    if ((operation === 'edit' || operation === 'extend') && !sourceAssetId) return notify(`${operation === 'edit' ? '编辑' : '延长'}需要先选择源视频`)
    const request: VideoRequest = { gatewayProfileId: 'mock-default', modelId: model, operation, prompt: prompt.trim(), durationSec, aspectRatio, resolution }
    try {
      setStatus('running'); setProgress(0)
      const adapter = gatewayRegistry.get('mock')
      const job = await adapter.createVideoJob(request)
      notify(`视频任务已创建 · ${job.id.slice(0, 8)}`)
      await jobManager.runVideoJob(job, ({ progress: nextProgress, status: nextStatus }) => { setProgress(nextProgress); if (nextStatus === 'succeeded') setStatus('succeeded') })
      notify('视频生成成功，可在视频库查看')
    } catch { setStatus('failed'); notify('视频任务失败，请重试') }
  }

  return <div className="creative-page studio-page video-studio-page"><div className="creative-header"><div><span className="eyebrow">视频工作区</span><h1>视频</h1></div></div><div className="notice-bar"><span>✧</span><span>从文字、首帧或参考图开始，生成一段完整视频。</span><button onClick={() => notify('视频能力说明已打开')}>查看说明 ↗</button></div><div className="canvas video-canvas"><div className="canvas-center"><span className="canvas-icon">▣</span><strong>{status === 'running' ? `生成中 ${progress}%` : status === 'succeeded' ? '视频任务已完成' : '描述你的想法，生成一段视频'}</strong><span>{status === 'succeeded' ? '结果会进入视频库' : '视频会显示在这里'}</span></div></div><div className="prompt-dock video-dock"><div className="operation-tabs">{(['generate','edit','extend'] as const).map(item => <button key={item} className={operation === item ? 'selected' : ''} onClick={() => { setOperation(item); setStatus('idle'); setProgress(0) }}>{item === 'generate' ? '生成' : item === 'edit' ? '编辑' : '延长'}</button>)}</div><textarea className="unified-input" value={prompt} onChange={e => setPrompt(e.target.value)} placeholder="描述视频画面、动作、镜头和节奏。" rows={3}/><div className="dock-controls"><select value={model} onChange={e => setModel(e.target.value)}><option>grok-imagine-video</option><option>veo-3</option><option>sora-2</option></select><button onClick={() => notify('首帧图选择器已打开')}>▧ 首帧图</button><button onClick={() => { setSourceAssetId('asset-demo'); notify('已选择示例源视频') }}>▧ 源视频</button><button onClick={() => notify('参考图选择器已打开')}>▧ 参考图</button><button onClick={() => notify('参考音色已切换')}>◖ 参考音色</button><select value={durationSec} onChange={e => setDurationSec(Number(e.target.value))}><option value={6}>6 秒</option><option value={12}>12 秒</option><option value={18}>18 秒</option></select><select value={aspectRatio} onChange={e => setAspectRatio(e.target.value)}><option>16:9</option><option>9:16</option><option>1:1</option></select><select value={resolution} onChange={e => setResolution(e.target.value)}><option>720p</option><option>1080p</option></select><button className="dock-submit" onClick={createJob} disabled={status === 'running'}>{status === 'running' ? `${progress}%` : '↑'}</button></div></div></div>
}
export default VideoPage
