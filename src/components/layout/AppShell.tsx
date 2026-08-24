import type { ReactNode } from 'react'
import type { Section, Theme, Notify } from '../../types/domain'
import { Sidebar } from './Sidebar'
import { Topbar } from './Topbar'

interface AppShellProps {
  section: Section
  theme: Theme
  resolvedTheme: 'light' | 'dark'
  toast: string
  notify: Notify
  onNavigate: (value: string) => void
  onThemeChange: (theme: Theme) => void
  children: ReactNode
}

export function AppShell({ section, theme, resolvedTheme, toast, notify, onNavigate, onThemeChange, children }: AppShellProps) {
  return <div className={`app-shell ${resolvedTheme} ${section === '渠道' ? 'channel-mode' : ''}`}>
    <Sidebar section={section} onNavigate={onNavigate} notify={notify} onSettings={() => onNavigate('设置')} />
    <main className="main-stage">
      <Topbar theme={theme} onThemeChange={onThemeChange} />
      <div className="content-scroll">{children}</div>
    </main>
    {toast && <div className="toast">✓ {toast}</div>}
  </div>
}
