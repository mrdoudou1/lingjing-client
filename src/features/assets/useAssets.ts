import { useCallback, useEffect, useMemo, useState } from 'react'
import type { Asset } from '../../types/domain'
import { assetRepository } from '../../services/assets/assetRepository'
import { subscribeToJobEvents } from '../../services/jobs/jobEvents'

export function useAssets() {
  const [assets, setAssets] = useState<Asset[]>([])
  const [filter, setFilter] = useState<'全部' | '最近生成' | '收藏'>('全部')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [usageBytes, setUsageBytes] = useState(0)
  const refresh = useCallback(async () => { setLoading(true); const [next, usage] = await Promise.all([assetRepository.list(), assetRepository.usage()]); setAssets(next); setUsageBytes(usage); setLoading(false) }, [])
  useEffect(() => {
    let mounted = true
    void Promise.all([assetRepository.list(), assetRepository.usage()]).then(([next, usage]) => {
      if (mounted) { setAssets(next); setUsageBytes(usage); setLoading(false) }
    })
    return () => { mounted = false }
  }, [])
  useEffect(() => {
    let mounted = true
    let stop: (() => void) | undefined
    void subscribeToJobEvents(() => {
      if (!mounted) return
      void assetRepository.reload().then(() => Promise.all([assetRepository.list(), assetRepository.usage()])).then(([next, usage]) => { if (mounted) { setAssets(next); setUsageBytes(usage) } })
    }).then(unlisten => { stop = unlisten })
    return () => { mounted = false; stop?.() }
  }, [])
  const visibleAssets = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    return assets.filter(asset => (filter !== '收藏' || asset.favorite) && (!normalized || `${asset.id} ${asset.kind} ${asset.mimeType}`.toLowerCase().includes(normalized)))
  }, [assets, filter, query])
  const remove = useCallback(async (id: string) => { await assetRepository.remove(id); await refresh() }, [refresh])
  const toggleFavorite = useCallback(async (id: string) => { await assetRepository.toggleFavorite(id); await refresh() }, [refresh])
  const openLocation = useCallback((id: string) => assetRepository.openLocation(id), [])
  const exportAsset = useCallback((id: string) => assetRepository.export(id), [])
  return { assets, visibleAssets, filter, setFilter, query, setQuery, loading, usageBytes, refresh, remove, toggleFavorite, openLocation, exportAsset }
}
