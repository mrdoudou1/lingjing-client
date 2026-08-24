import type { Theme } from '../../types/domain'

interface TopbarProps { theme: Theme; onThemeChange: (theme: Theme) => void }
export function Topbar({ theme, onThemeChange }: TopbarProps) {
  return <header className="topbar"><div className="top-actions"><div className="theme-pill" aria-label="主题选择"><button className={theme === 'system' ? 'selected' : ''} onClick={() => onThemeChange('system')}>⌁ 跟随系统</button><button className={theme === 'light' ? 'selected' : ''} onClick={() => onThemeChange('light')}>☼ 浅色</button><button className={theme === 'dark' ? 'selected' : ''} onClick={() => onThemeChange('dark')}>☾ 深色</button></div><div className="avatar small">A</div></div></header>
}
