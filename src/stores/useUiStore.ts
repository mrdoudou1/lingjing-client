import { useEffect, useRef, useState } from 'react'
import type { Section, Theme, Notify } from '../types/domain'
import { tauriBridge } from '../services/tauri/bridge'
import { appPersistence } from '../services/persistence/persistence'

export function useUiStore(initialSection: Section = '图片') {
  const [section, setSection] = useState<Section>(initialSection)
  const [theme, setTheme] = useState<Theme>('system')
  const [systemDark, setSystemDark] = useState(false)
  const [toast, setToast] = useState('')
  const toastTimer = useRef<number | undefined>(undefined)

  useEffect(() => {
    let mounted = true
    const load = async () => {
      try {
        const settings = tauriBridge.available()
          ? await tauriBridge.invoke<Record<string, unknown>>('settings_get')
          : await appPersistence.get<Record<string, unknown>>('settings')
        const savedTheme = settings?.theme
        const savedSection = settings?.lastSection
        if (!mounted) return
        if (savedTheme === 'system' || savedTheme === 'light' || savedTheme === 'dark') setTheme(savedTheme)
        if (typeof savedSection === 'string') setSection(savedSection as Section)
      } catch { /* use defaults when persistence is unavailable */ }
    }
    void load()
    return () => { mounted = false }
  }, [])

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const update = () => setSystemDark(media.matches)
    update()
    media.addEventListener('change', update)
    return () => media.removeEventListener('change', update)
  }, [])

  useEffect(() => {
    const values = { theme, lastSection: section }
    if (tauriBridge.available()) void tauriBridge.invoke('settings_update', { update: { values } }).catch(() => {})
    else void appPersistence.set('settings', values)
  }, [section, theme])

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
