import type { ImageRequest, ModelCapabilities } from '../../types/domain'

export type ImageValidationResult = { ok: true } | { ok: false; message: string }

export function validateImageRequest(request: ImageRequest, capabilities: ModelCapabilities): ImageValidationResult {
  const image = capabilities.image
  if (!image) return { ok: false, message: '当前模型不支持图片生成' }
  if (!request.prompt.trim()) return { ok: false, message: '请输入图片描述' }
  const min = image.count?.min ?? 1
  const max = image.count?.max ?? 1
  if (!Number.isInteger(request.count) || request.count < min || request.count > max) return { ok: false, message: `生成数量需在 ${min} 到 ${max} 张之间` }
  if (request.aspectRatio && !image.aspectRatios.includes(request.aspectRatio)) return { ok: false, message: '当前模型不支持该比例' }
  if (request.resolution && !image.resolutions.includes(request.resolution)) return { ok: false, message: '当前模型不支持该分辨率' }
  if (request.quality && !image.qualities.includes(request.quality)) return { ok: false, message: '当前模型不支持该质量' }
  return { ok: true }
}
