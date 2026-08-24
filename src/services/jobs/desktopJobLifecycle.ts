import type { JobStatus } from '../../types/domain'
import type { GenerationJob } from '../../types/domain'
import { tauriBridge } from '../tauri/bridge'

export function updateDesktopJob(id: string, status: JobStatus, progress: number, errorMessage?: string) {
  if (!tauriBridge.available()) return
  void tauriBridge.invoke('job_update', { id, status, progress, errorMessage }).catch(() => {})
}

export function cancelDesktopJob(id: string) {
  if (!tauriBridge.available()) return
  void tauriBridge.invoke('job_cancel', { id }).catch(() => {})
}

export async function pollDesktopVideoJob<TRequest>(id: string, request: TRequest): Promise<GenerationJob<TRequest> | null> {
  if (!tauriBridge.available()) return null
  const raw = await tauriBridge.invoke<Record<string, unknown> | null>('video_poll_job', { id })
  if (!raw) return null
  return {
    id: String(raw.id ?? id),
    kind: 'video',
    status: String(raw.status ?? 'running') as GenerationJob<TRequest>['status'],
    progress: Number(raw.progress ?? 0),
    request,
    errorMessage: raw.error_message ? String(raw.error_message) : undefined,
    createdAt: String(raw.created_at ?? new Date().toISOString()),
  }
}
