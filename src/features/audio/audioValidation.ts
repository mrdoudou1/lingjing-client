import type { AudioRequest, ModelCapabilities } from '../../types/domain'

export type AudioValidationResult = { ok: true } | { ok: false; message: string }

export function validateAudioRequest(request: AudioRequest, capabilities: ModelCapabilities): AudioValidationResult {
  if (request.kind === 'tts') {
    if (!capabilities.tts) return { ok: false, message: '当前模型不支持语音合成' }
    if (!request.text?.trim()) return { ok: false, message: '请输入需要合成的文本' }
    if (!capabilities.tts.formats.includes(request.format)) return { ok: false, message: '当前模型不支持该音频格式' }
    if (request.voice && !capabilities.tts.voices.includes(request.voice)) return { ok: false, message: '当前模型不支持该声音' }
  } else {
    if (!capabilities.stt) return { ok: false, message: '当前模型不支持语音识别' }
    if (!request.sourceFileName) return { ok: false, message: '请先选择音频或视频文件' }
    if (!capabilities.stt.formats.includes(request.format)) return { ok: false, message: '当前模型不支持该输出格式' }
    if (request.sourceFileName && !/\.(mp3|wav|m4a|mp4|mov|webm|ogg)$/i.test(request.sourceFileName)) return { ok: false, message: '文件格式不受支持' }
  }
  return { ok: true }
}
