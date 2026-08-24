import { useEffect, useRef, useState } from 'react'
import type { Section, Theme, Notify } from '../types/domain'

export function useUiStore(initialSection: Section = '图片') {
  const [section, setSection] = useState<Section>(initialSection)
  const [theme, setTheme] = useState<Theme>('system')
  const [systemDark, setSystemDark] = useState(false)
  const [toast, setToast] = useState('')
  const toastTimer = useRef<number | undefined>(undefined)

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const update = () => setSystemDark(media.matches)
    update()
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])

  useEffect(() => () => {
    if (toastTimer.current) window.clearTimeout(toastTimer.current)
  }, [])

  const notify: Notify = (message) => {
    setToast(message)
    if (toastTimer.current) window.clearTimeout(toastTimer.current)
    toastTimer.current = window.setTimeout(() => setToast(''), 2200)
  }

  const resolvedTheme = theme === 'system' ? (systemDark ? 'dark' : 'light') : theme
  return { section, setSection, theme, setTheme, resolvedTheme, toast, notify }
}
