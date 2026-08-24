import type { Section } from '../types/domain'

export const primaryNav = [['聊天', '▤'], ['图片', '▧'], ['视频', '▣'], ['TTS / STT', '◖'], ['素材库', '▧'], ['历史记录', '◷']] as const
export const managementNav = [['视频库', '▣'], ['渠道', '◌']] as const

export function resolveSection(value: string): Section | null {
  if (value === '语音') return 'TTS / STT'
  if (value === '素材库') return '素材库'
  if (['聊天', '图片', '视频', 'TTS / STT', '图库', '视频库', '渠道', '历史记录', '设置'].includes(value)) return value as Section
  return null
}
