import { useCallback, useEffect, useState } from 'react'
import type { Notify, VideoOperation } from '../../types/domain'
import { gatewayRegistry } from '../../services/gateway/registry'
import type { VideoRequest } from '../../services/gateway/types'
import { validateVideoRequest } from './videoValidation'
import { VideoJobService } from '../../services/jobs/videoJobService'

const adapter = gatewayRegistry.runtime()
const videoJobs = new VideoJobService(adapter)

export function useVideoWorkspace(notify: Notify) {
  const [operation, setOperation] = useState<VideoOperation>('generate')
  const [prompt, setPrompt] = useState('')
  const [model, setModel] = useState('grok-imagine-video')
  const [durationSec, setDurationSec] = useState(6)
  const [aspectRatio, setAspectRatio] = useState('16:9')
  const [resolution, setResolution] = useState('720p')
  const [sourceVideoAssetId, setSourceVideoAssetId] = useState('')
  const [firstFrameAssetId, setFirstFrameAssetId] = useState('')
  const [referenceImageAssetIds, setReferenceImageAssetIds] = useState<string[]>([])
  const [referenceVoiceIds, setReferenceVoiceIds] = useState<string[]>([])
  const [jobId, setJobId] = useState('')
  const [progress, setProgress] = useState(0)
  const [status, setStatus] = useState<'idle' | 'running' | 'succeeded' | 'failed' | 'canceled'>('idle')

  useEffect(() => () => { if (jobId && status === 'running') videoJobs.cancel(jobId) }, [jobId, status])
  const selectOperation = useCallback((next: VideoOperation) => { setOperation(next); setStatus('idle'); setProgress(0) }, [])
  const selectFirstFrame = useCallback(() => { setFirstFrameAssetId('asset-first-frame-demo'); setReferenceImageAssetIds([]); notify('已选择示例首帧图') }, [notify])
  const selectReferenceImage = useCallback(() => { setReferenceImageAssetIds(['asset-reference-demo']); setFirstFrameAssetId(''); notify('已选择示例参考图') }, [notify])
  const toggleReferenceVoice = useCallback(() => { setReferenceVoiceIds(previous => previous.length ? [] : ['voice-demo']); notify(referenceVoiceIds.length ? '已移除参考音色' : '已选择示例参考音色') }, [notify, referenceVoiceIds.length])
  const start = useCallback(async () => {
    const request: VideoRequest = { gatewayProfileId: 'mock-default', modelId: model, operation, inputMode: firstFrameAssetId ? 'image-to-video' : referenceImageAssetIds.length ? 'reference-to-video' : 'text-to-video', prompt, firstFrameAssetId: firstFrameAssetId || undefined, referenceImageAssetIds, referenceVoiceIds, sourceVideoAssetId: sourceVideoAssetId || undefined, durationSec, extensionDurationSec: operation === 'extend' ? durationSec : undefined, aspectRatio, resolution }
    const capabilities = await adapter.resolveCapabilities(request.modelId)
    const validation = validateVideoRequest(request, capabilities)
    if (!validation.ok) return notify(validation.message)
    setStatus('running'); setProgress(0)
    try {
      const result = await videoJobs.start(request, state => { setProgress(state.progress); setStatus(state.status === 'canceled' ? 'canceled' : state.status === 'succeeded' ? 'succeeded' : 'running') }, setJobId)
      setJobId(result.id)
      if (result.status === 'succeeded') notify('视频生成成功，结果已写入素材库')
      if (result.status === 'canceled') notify('视频任务已取消')
    } catch { setStatus('failed'); notify('视频任务失败，请重试') }
  }, [aspectRatio, durationSec, firstFrameAssetId, model, notify, operation, prompt, referenceImageAssetIds, referenceVoiceIds, resolution, sourceVideoAssetId])
  const cancel = useCallback(() => { if (jobId) videoJobs.cancel(jobId) }, [jobId])
  const retry = useCallback(async () => { if (!jobId) return; setStatus('running'); setProgress(0); try { const result = await videoJobs.retry(jobId, state => { setProgress(state.progress); setStatus(state.status === 'canceled' ? 'canceled' : state.status === 'succeeded' ? 'succeeded' : 'running') }, setJobId); if (result.status === 'succeeded') notify('视频任务重试完成') } catch { setStatus('failed'); notify('视频重试失败') } }, [jobId, notify])
  return { operation, setOperation: selectOperation, prompt, setPrompt, model, setModel, durationSec, setDurationSec, aspectRatio, setAspectRatio, resolution, setResolution, sourceVideoAssetId, setSourceVideoAssetId, progress, status, jobId, referenceImageAssetIds, referenceVoiceIds, selectFirstFrame, selectReferenceImage, toggleReferenceVoice, start, cancel, retry }
}
