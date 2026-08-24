import type { GenerationJob, JobStatus } from '../../types/domain'
import type { VideoRequest } from '../gateway/types'

export interface JobProgress { status: JobStatus; progress: number }

export class MockJobManager {
  async runVideoJob(job: GenerationJob<VideoRequest>, onProgress: (state: JobProgress) => void) {
    for (let progress = 20; progress <= 100; progress += 20) {
      await new Promise((resolve) => window.setTimeout(resolve, 120))
      onProgress({ status: progress === 100 ? 'succeeded' : 'running', progress })
    }
    return { ...job, status: 'succeeded' as const, progress: 100 }
  }
}
export const jobManager = new MockJobManager()
