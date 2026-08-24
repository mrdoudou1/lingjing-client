import type { Section } from '../types/domain'

export const workspaceNav = [
  ['仪表盘', '▦'], ['图库', '▧'], ['视频库', '▣'], ['请求审计', '◉'], ['质量守护', '♢'],
] as const
export const studioNav = [['聊天', '▤'], ['图片', '▧'], ['视频', '▣'], ['语音', '◖']] as const

export function resolveSection(value: string): Section | null {
  if (value === '语音') return 'TTS / STT'
  if (['聊天', '图片', '视频', '图库', '视频库', '渠道', '历史记录', '设置'].includes(value)) return value as Section
  return null
}
