import './App.css'
import { AppShell } from './components/layout/AppShell'
import { resolveSection } from './app/navigation'
import { useUiStore } from './stores/useUiStore'
import type { Section } from './types/domain'
import { ChatPage } from './pages/ChatPage'
import { ImagePage } from './pages/ImagePage'
import VideoPage from './pages/VideoPage'
import { AudioPage } from './pages/AudioPage'
import { AssetsPage } from './pages/AssetsPage'
import { VideoLibraryPage } from './pages/VideoLibraryPage'
import { ChannelPage } from './pages/ChannelPage'
import { HistoryPage } from './pages/HistoryPage'
import { SettingsPage } from './pages/SettingsPage'

function App() {
  const ui = useUiStore('图片')
  const navigate = (value: string) => {
    const next = resolveSection(value)
    if (next) ui.setSection(next)
    else ui.notify(`${value} 模块将在后续版本接入`)
  }
  const renderPage = (section: Section) => {
    switch (section) {
      case '聊天': return <ChatPage notify={ui.notify} />
      case '图片': return <ImagePage notify={ui.notify} />
      case '视频': return <VideoPage notify={ui.notify} />
      case 'TTS / STT': return <AudioPage notify={ui.notify} />
      case '图库': return <AssetsPage notify={ui.notify} />
      case '视频库': return <VideoLibraryPage notify={ui.notify} />
      case '渠道': return <ChannelPage notify={ui.notify} />
      case '历史记录': return <HistoryPage notify={ui.notify} />
      case '设置': return <SettingsPage theme={ui.theme} setTheme={ui.setTheme} notify={ui.notify} />
    }
  }
  return <AppShell section={ui.section} theme={ui.theme} resolvedTheme={ui.resolvedTheme} toast={ui.toast} notify={ui.notify} onNavigate={navigate} onThemeChange={ui.setTheme}>{renderPage(ui.section)}</AppShell>
}
export default App
