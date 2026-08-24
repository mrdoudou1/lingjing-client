import { useCallback, useEffect, useMemo, useState } from 'react'
import type { HistoryRecord } from '../../types/domain'
import { historyRepository } from '../../services/jobs/historyRepository'
import { subscribeToJobEvents } from '../../services/jobs/jobEvents'

export function useHistory() {
  const [records, setRecords] = useState<HistoryRecord[]>([])
  const [filter, setFilter] = useState<'全部' | HistoryRecord['kind']>('全部')
  const [loading, setLoading] = useState(true)
  const refresh = useCallback(async () => { setLoading(true); setRecords(await historyRepository.list()); setLoading(false) }, [])
  useEffect(() => { let mounted = true; void historyRepository.list().then(next => { if (mounted) { setRecords(next); setLoading(false) } }); return () => { mounted = false } }, [])
  useEffect(() => {
    let mounted = true
    let stop: (() => void) | undefined
    void subscribeToJobEvents(() => {
      if (!mounted) return
      void historyRepository.list().then(next => { if (mounted) setRecords(next) })
    }).then(unlisten => { stop = unlisten })
    return () => { mounted = false; stop?.() }
  }, [])
  const visible = useMemo(() => filter === '全部' ? records : records.filter(record => record.kind === filter), [filter, records])
  return { records, visible, filter, setFilter, loading, refresh }
}
