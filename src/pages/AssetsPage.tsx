import type { Asset, Notify } from '../types/domain'
import { PageHeader } from '../components/common/PageHeader'
import { VirtualGallery } from '../components/common/VirtualGallery'
import { useState } from 'react'
import { useAssets } from '../features/assets/useAssets'
import { AssetPreviewDialog } from '../components/assets/AssetPreviewDialog'

const gradients = ['gallery-a', 'gallery-b', 'gallery-c', 'gallery-d', 'gallery-e', 'gallery-f', 'gallery-g', 'gallery-h', 'gallery-i', 'gallery-j', 'gallery-k', 'gallery-l', 'gallery-m', 'gallery-n', 'gallery-o']

function AssetCard({ asset, index, library, notify, onPreview }: { asset: Asset; index: number; library: ReturnType<typeof useAssets>; notify: Notify; onPreview: (asset: Asset) => void }) {
  return <div className="gallery-card asset-card"><button onClick={() => onPreview(asset)}><div className={`gallery-image ${gradients[index % gradients.length]}`}><span>{asset.kind === 'video' ? '▶' : asset.kind === 'audio' ? '◖' : '✦'}</span></div><div className="gallery-meta"><strong>{asset.kind === 'video' ? '视频资产' : asset.kind === 'audio' ? '音频资产' : '图片资产'}</strong><span>{asset.id.slice(0, 16)}</span><small>{asset.mimeType} · {Math.round(asset.sizeBytes / 1024)} KB</small></div></button><div className="asset-actions"><button onClick={() => { void library.toggleFavorite(asset.id); notify(asset.favorite ? '已取消收藏' : '已收藏') }}>{asset.favorite ? '★' : '☆'}</button><button onClick={() => { void library.openLocation(asset.id).then(() => notify('已打开文件位置')).catch(error => notify(error instanceof Error ? error.message : '打开失败')) }}>位置</button><button onClick={() => { void library.exportAsset(asset.id).then(() => notify('已导出到下载目录')).catch(error => notify(error instanceof Error ? error.message : '导出失败')) }}>下载</button><button className="asset-delete" onClick={() => { void library.remove(asset.id); notify('资产已删除') }}>删除</button></div></div>
}

export function AssetsPage({ notify }: { notify: Notify }) {
  const library = useAssets()
  const [previewAsset, setPreviewAsset] = useState<Asset>()
  const usageMb = Math.round((library.usageBytes / 1024 / 1024) * 10) / 10
  return <div className="library-page"><PageHeader eyebrow="本地资产" title="图库" actions={<button className="light-button" onClick={() => { void library.refresh(); notify('图库已刷新') }}>↻ 刷新</button>} /><div className="library-stats"><div className="search-box">⌕<input value={library.query} onChange={event => library.setQuery(event.target.value)} placeholder="搜索 ID、类型或哈希" /></div><div className="stats"><span>▧ 资产总数 <strong>{library.assets.length}</strong></span><span>▤ 占用空间 <strong>{usageMb} MB</strong></span></div></div><div className="filter-row">{(['全部', '最近生成', '收藏'] as const).map(item => <button key={item} className={library.filter === item ? 'selected' : ''} onClick={() => { library.setFilter(item); notify(`已切换${item}`) }}>{item}</button>)}</div>{library.loading ? <div className="history-placeholder"><span>◌</span><h2>正在加载素材</h2><p>从本地资产仓库读取缩略图。</p></div> : library.visibleAssets.length === 0 ? <div className="history-placeholder"><span>▧</span><h2>暂无素材</h2><p>完成图片、视频或音频任务后，生成文件会显示在这里。</p></div> : <VirtualGallery items={library.visibleAssets} renderItem={(asset, index) => <AssetCard asset={asset} index={index} library={library} notify={notify} onPreview={setPreviewAsset} />} />} {previewAsset && <AssetPreviewDialog asset={previewAsset} onClose={() => setPreviewAsset(undefined)} />}</div>
}

