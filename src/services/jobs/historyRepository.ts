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
  const { source_file_base64: _sourceFileBase64, sourceFileBase64: _sourceFileBase64Camel, ...safeRequest } = request
  return {
    ...safeRequest,
    gatewayProfileId: safeRequest.gateway_profile_id ?? safeRequest.gatewayProfileId,
    modelId: safeRequest.model_id ?? safeRequest.modelId,
    sourceFileName: safeRequest.source_file_name ?? safeRequest.sourceFileName,
    referenceAssetIds: safeRequest.reference_asset_ids ?? safeRequest.referenceAssetIds,
    referenceImageAssetIds: safeRequest.reference_image_asset_ids ?? safeRequest.referenceImageAssetIds,
    referenceVoiceIds: safeRequest.reference_voice_ids ?? safeRequest.referenceVoiceIds,
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
    const rawRequest = job.request && typeof job.request === 'object' ? job.request as Record<string, unknown> : {}
    const { sourceFileBase64: _sourceFileBase64, source_file_base64: _sourceFileBase64Snake, ...request } = rawRequest
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
