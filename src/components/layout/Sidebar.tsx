import type { Notify, Section } from '../../types/domain'
import { managementNav, primaryNav } from '../../app/navigation'

interface SidebarProps {
  section: Section
  onNavigate: (value: string) => void
  onSettings: () => void
  notify: Notify
}

export function Sidebar({ section, onNavigate, onSettings, notify }: SidebarProps) {
  return <aside className="sidebar">
    <div className="brand-row"><div className="brand-mark">L</div><div><strong>Lingjing</strong><span>v0.1.0 · 本地 AI 工作区</span></div><button className="github-button" onClick={() => notify('项目链接已复制')}>◉</button></div>
    <div className="side-section">
      <span className="side-label">工作区</span>
      {primaryNav.map(([label, icon]) => <button key={label} className={`side-item ${section === label || (label === '素材库' && section === '图库') ? 'active' : ''}`} onClick={() => onNavigate(label)}><span>{icon}</span>{label}<b>›</b></button>)}
      <span className="side-label side-label-secondary">管理</span>
      {managementNav.map(([label, icon]) => <button key={label} className={`side-item ${section === label ? 'active' : ''}`} onClick={() => onNavigate(label)}><span>{icon}</span>{label}</button>)}
    </div>
    <div className="sidebar-footer"><div className="profile-line"><div className="avatar">A</div><span>Admin</span><button onClick={() => notify('更多账户操作')}>•••</button><button onClick={onSettings}>⚙</button></div><div className="usage-line"><span>本地工作区</span><strong>68%</strong></div></div>
  </aside>
}
