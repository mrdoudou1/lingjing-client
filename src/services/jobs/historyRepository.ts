import type { GenerationJob, HistoryRecord, JobStatus } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'
import { tauriBridge } from '../tauri/bridge'

type RustGenerationJob = {
  id: string
  gateway_profile_id: string
  kind: HistoryRecord['kind']
  model_id?: string
  status: JobStatus
  progress: number
  request_json: Record<string, unknown>
  error_message?: string
  created_at: string
}

function normalizeRequest(request: Record<string, unknown>) {
  return {
    ...request,
    gatewayProfileId: request.gateway_profile_id ?? request.gatewayProfileId,
    modelId: request.model_id ?? request.modelId,
    sourceFileName: request.source_file_name ?? request.sourceFileName,
    referenceAssetIds: request.reference_asset_ids ?? request.referenceAssetIds,
    referenceImageAssetIds: request.reference_image_asset_ids ?? request.referenceImageAssetIds,
    referenceVoiceIds: request.reference_voice_ids ?? request.referenceVoiceIds,
  }
}

function fromRustJob(job: RustGenerationJob): HistoryRecord {
  const terminal = ['succeeded', 'failed', 'canceled', 'stopped'].includes(job.status)
  const request = normalizeRequest(job.request_json)
  return {
    id: 'history_' + job.id,
    jobId: job.id,
    kind: job.kind,
    status: job.status,
    modelId: job.model_id ?? String(request.modelId ?? ''),
    gatewayProfileId: job.gateway_profile_id,
    request,
    createdAt: job.created_at,
    finishedAt: terminal ? job.created_at : undefined,
    errorMessage: job.error_message,
  }
}

class HistoryRepository {
  private records: HistoryRecord[] = []
  private loaded = false

  private async ensureLoaded() {
    if (!this.loaded) {
      this.records = (await appPersistence.get<HistoryRecord[]>('history')) ?? []
      this.loaded = true
    }
  }

  async list() {
    if (tauriBridge.available()) {
      try {
        const jobs = await tauriBridge.invoke<RustGenerationJob[]>('job_list', { kind: null, status: null })
        this.records = jobs.map(fromRustJob)
        this.loaded = true
        return [...this.records].sort((a, b) => b.createdAt.localeCompare(a.createdAt))
      } catch { /* browser persistence remains the fallback */ }
    }
    await this.ensureLoaded()
    return [...this.records].sort((a, b) => b.createdAt.localeCompare(a.createdAt))
  }

  async save(record: HistoryRecord) {
    await this.ensureLoaded()
    if (tauriBridge.available()) {
      const progress = ['succeeded', 'failed', 'canceled', 'stopped'].includes(record.status) ? 100 : 0
      await tauriBridge.invoke('job_update', {
        id: record.jobId,
        status: record.status,
        progress,
        errorMessage: record.errorMessage,
      })
    } else {
      this.records = [record, ...this.records.filter(item => item.id !== record.id)]
      await appPersistence.set('history', this.records)
    }
    this.records = [record, ...this.records.filter(item => item.id !== record.id)]
  }

  async recordJob<T>(job: GenerationJob<T>, status: HistoryRecord['status'], errorMessage?: string) {
    const request = job.request && typeof job.request === 'object' ? job.request as Record<string, unknown> : {}
    await this.save({
      id: 'history_' + job.id,
      jobId: job.id,
      kind: job.kind,
      status,
      modelId: String(request.modelId ?? request.model_id ?? ''),
      gatewayProfileId: String(request.gatewayProfileId ?? request.gateway_profile_id ?? ''),
      request,
      createdAt: job.createdAt,
      finishedAt: new Date().toISOString(),
      errorMessage,
    })
  }
}

export const historyRepository = new HistoryRepository()
