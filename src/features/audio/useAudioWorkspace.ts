import { useCallback, useState } from 'react'
import type { AudioRequest, ModelCapabilities, Notify } from '../../types/domain'
import { gatewayRegistry } from '../../services/gateway/registry'
import { AudioJobService } from '../../services/jobs/audioJobService'
import { validateAudioRequest } from './audioValidation'

const adapter = gatewayRegistry.get('mock')
const audioJobs = new AudioJobService(adapter)
const audioCapabilities: ModelCapabilities = { tts: { voices: ['Aria · 温暖女声', 'River · 平静男声'], formats: ['MP3', 'WAV'], streaming: false }, stt: { languages: ['中文（普通话）', 'English'], formats: ['TXT', 'JSON', 'SRT', 'VTT'], timestamps: true, realtime: false } }

export function useAudioWorkspace(notify: Notify) {
  const [tab, setTab] = useState<'tts' | 'stt'>('tts')
  const [text, setText] = useState('')
  const [voice, setVoice] = useState('Aria · 温暖女声')
  const [language, setLanguage] = useState('中文（普通话）')
  const [format, setFormat] = useState('MP3')
  const [sourceFile, setSourceFile] = useState<File | null>(null)
  const [status, setStatus] = useState<'idle' | 'running' | 'succeeded' | 'failed'>('idle')
  const [progress, setProgress] = useState(0)
  const request = useCallback((): AudioRequest => tab === 'tts' ? { gatewayProfileId: 'mock-default', modelId: 'mock-audio', kind: 'tts', text, voice, format } : { gatewayProfileId: 'mock-default', modelId: 'mock-audio', kind: 'stt', sourceFileName: sourceFile?.name, language, format }, [format, language, sourceFile, tab, text, voice])
  const submit = useCallback(async () => {
    const input = request(); const validation = validateAudioRequest(input, audioCapabilities)
    if (!validation.ok) return notify(validation.message)
    setStatus('running'); setProgress(0)
    try { const result = await audioJobs.start(input, state => { setProgress(state.progress); setStatus(state.status === 'succeeded' ? 'succeeded' : 'running') }); if (result.status === 'succeeded') notify(tab === 'tts' ? '语音合成成功，已保存到素材库' : '语音识别成功，已保存到历史记录') } catch { setStatus('failed'); notify('音频任务失败，请重试') }
  }, [notify, request, tab])
  const chooseFile = useCallback((file: File | undefined) => { if (file) { setSourceFile(file); notify(`已选择文件：${file.name}`) } }, [notify])
  return { tab, setTab: (next: 'tts' | 'stt') => { setTab(next); setStatus('idle'); setProgress(0); setFormat(next === 'tts' ? 'MP3' : 'SRT') }, text, setText, voice, setVoice, language, setLanguage, format, setFormat, sourceFile, chooseFile, status, progress, submit }
}
