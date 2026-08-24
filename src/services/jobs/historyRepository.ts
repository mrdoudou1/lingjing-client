import type { GenerationJob, HistoryRecord } from '../../types/domain'
import { appPersistence } from '../persistence/persistence'

class HistoryRepository {
  private records: HistoryRecord[] = []
  private loaded = false
  private async ensureLoaded() { if (!this.loaded) { this.records = (await appPersistence.get<HistoryRecord[]>('history')) ?? []; this.loaded = true } }
  async list() { await this.ensureLoaded(); return [...this.records].sort((a, b) => b.createdAt.localeCompare(a.createdAt)) }
  async save(record: HistoryRecord) { await this.ensureLoaded(); this.records = [record, ...this.records.filter(item => item.id !== record.id)]; await appPersistence.set('history', this.records) }
  async recordJob<T>(job: GenerationJob<T>, status: HistoryRecord['status'], errorMessage?: string) { await this.save({ id: `history_${job.id}`, jobId: job.id, kind: job.kind, status, modelId: String(job.request && typeof job.request === 'object' && 'modelId' in job.request ? job.request.modelId : ''), gatewayProfileId: String(job.request && typeof job.request === 'object' && 'gatewayProfileId' in job.request ? job.request.gatewayProfileId : ''), request: job.request, createdAt: job.createdAt, finishedAt: new Date().toISOString(), errorMessage }) }
}
export const historyRepository = new HistoryRepository()
