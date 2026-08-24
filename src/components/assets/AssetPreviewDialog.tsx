import { useMemo } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { Asset } from '../../types/domain'
import { tauriBridge } from '../../services/tauri/bridge'

export function AssetPreviewDialog({ asset, onClose }: { asset: Asset; onClose: () => void }) {
  const source = useMemo(() => asset.localPath.startsWith('mock://') ? undefined : tauriBridge.available() ? convertFileSrc(asset.localPath) : asset.localPath, [asset.localPath])
  const preview = !source ? <div className="asset-preview-empty"><strong>暂无可预览文件</strong><span>该资产是 Mock 占位或本地文件已不存在。</span></div> : asset.kind === 'image' ? <img src={source} alt={asset.id} /> : asset.kind === 'video' ? <video src={source} controls autoPlay={false} /> : <audio src={source} controls autoPlay={false} />
  return <div className="asset-preview-backdrop" role="presentation" onClick={onClose}><section className="asset-preview-dialog" role="dialog" aria-modal="true" aria-label="资产预览" onClick={event => event.stopPropagation()}><header><div><strong>{asset.kind === 'image' ? '图片预览' : asset.kind === 'video' ? '视频预览' : '音频预览'}</strong><small>{asset.mimeType} · {Math.round(asset.sizeBytes / 1024)} KB</small></div><button onClick={onClose} aria-label="关闭预览">×</button></header><div className="asset-preview-content">{preview}</div></section></div>
}
