import type { JobStatus } from '../../types/domain'
import { tauriBridge } from '../tauri/bridge'

export function updateDesktopJob(id: string, status: JobStatus, progress: number, errorMessage?: string) {
  if (!tauriBridge.available()) return
  void tauriBridge.invoke('job_update', { id, status, progress, errorMessage }).catch(() => {})
}

export function cancelDesktopJob(id: string) {
  if (!tauriBridge.available()) return
  void tauriBridge.invoke('job_cancel', { id }).catch(() => {})
}
