import { useCallback, useState } from 'react'
import type { ImageRequest, Notify } from '../../types/domain'
import { gatewayRegistry } from '../../services/gateway/registry'
import { ImageJobService } from '../../services/jobs/imageJobService'
import { validateImageRequest } from './imageValidation'
import { useModelCapabilities } from '../gateways/useModelCapabilities'

const adapter = gatewayRegistry.runtime()
const imageJobs = new ImageJobService(adapter)

export function useImageWorkspace(notify: Notify) {
  const [prompt, setPrompt] = useState('')
  const [model, setModel] = useState('grok-imagine-image-2.0')
  const [count, setCount] = useState(1)
  const [aspectRatio, setAspectRatio] = useState('1 : 1')
  const [resolution, setResolution] = useState<'1k' | '2k'>('1k')
  const [quality, setQuality] = useState('standard')
  const [referenceAssetIds, setReferenceAssetIds] = useState<string[]>([])
  const [status, setStatus] = useState<'idle' | 'running' | 'succeeded' | 'failed'>('idle')
  const [progress, setProgress] = useState(0)
  const [jobId, setJobId] = useState('')
  const capabilityState = useModelCapabilities(model)
  const selectModel = useCallback(async (nextModel: string) => {
    setModel(nextModel)
    const image = (await adapter.resolveCapabilities(nextModel)).image
    if (!image) return
    setCount(previous => image.count ? Math.min(Math.max(previous, image.count.min), image.count.max) : previous)
    setAspectRatio(previous => image.aspectRatios.includes(previous) ? previous : image.aspectRatios[0] ?? previous)
    setResolution(previous => image.resolutions.includes(previous) ? previous : (image.resolutions[0] as '1k' | '2k' | undefined) ?? previous)
    setQuality(previous => image.qualities.includes(previous) ? previous : image.qualities[0] ?? previous)
    notify(`已切换模型：${nextModel}，参数已按能力校正`)
  }, [notify])
  const selectReference = useCallback(() => { setReferenceAssetIds(['asset-reference-image-demo']); notify('已选择示例参考图') }, [notify])
  const start = useCallback(async () => {
    const request: ImageRequest = { gatewayProfileId: 'mock-default', modelId: model, prompt, count, aspectRatio, resolution, quality, referenceAssetIds }
    const validation = validateImageRequest(request, await adapter.resolveCapabilities(model))
    if (!validation.ok) return notify(validation.message)
    setStatus('running'); setProgress(0)
    try {
      const result = await imageJobs.start(request, state => { setProgress(state.progress); setStatus(state.status === 'succeeded' ? 'succeeded' : 'running') }, setJobId)
      if (result.status === 'succeeded') notify(`${count} 张图片生成成功，已写入素材库`)
    } catch { setStatus('failed'); notify('图片任务失败，请重试') }
  }, [aspectRatio, count, model, notify, prompt, quality, referenceAssetIds, resolution])
  return { prompt, setPrompt, model, setModel, selectModel, count, setCount, aspectRatio, setAspectRatio, resolution, setResolution, quality, setQuality, referenceAssetIds, status, progress, jobId, capabilities: capabilityState.capabilities.image, capabilitiesLoading: capabilityState.loading, selectReference, start }
}
