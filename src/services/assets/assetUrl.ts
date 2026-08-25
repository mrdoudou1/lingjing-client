import { convertFileSrc } from '@tauri-apps/api/core'
import { tauriBridge } from '../tauri/bridge'

/** Convert a local asset path into a browser-playable URL in Tauri or web mode. */
export function assetUrl(localPath: string): string | undefined {
  if (!localPath || localPath.startsWith('mock://')) return undefined
  return tauriBridge.available() ? convertFileSrc(localPath) : localPath
}
