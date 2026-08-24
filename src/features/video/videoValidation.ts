import type { ModelCapabilities } from '../../types/domain'
import type { VideoRequest } from '../../services/gateway/types'

export type VideoValidationResult = { ok: true } | { ok: false; message: string }

export function validateVideoRequest(request: VideoRequest, capabilities: ModelCapabilities): VideoValidationResult {
  const video = capabilities.video
  if (!video?.operations.includes(request.operation)) return { ok: false, message: '当前模型不支持该视频操作' }
  if (!request.prompt.trim()) return { ok: false, message: '请输入视频描述' }
  if (request.firstFrameAssetId && request.referenceImageAssetIds?.length) return { ok: false, message: '首帧图与参考图不能同时使用' }
  if ((request.operation === 'edit' || request.operation === 'extend') && !request.sourceVideoAssetId) return { ok: false, message: '编辑或延长需要先选择源视频' }
  if ((request.referenceImageAssetIds?.length ?? 0) > (video.maxReferenceImages ?? 0)) return { ok: false, message: '参考图数量超过模型上限' }
  if ((request.referenceVoiceIds?.length ?? 0) > (video.maxReferenceVoices ?? 0)) return { ok: false, message: '参考音色数量超过模型上限' }
  if (request.durationSec && video.durations && !video.durations.includes(request.durationSec)) return { ok: false, message: '当前模型不支持该时长' }
  if (request.aspectRatio && !video.aspectRatios.includes(request.aspectRatio)) return { ok: false, message: '当前模型不支持该画幅' }
  if (request.resolution && !video.resolutions.includes(request.resolution)) return { ok: false, message: '当前模型不支持该分辨率' }
  return { ok: true }
}
