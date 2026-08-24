import { useCallback, useEffect, useMemo, useState } from 'react'
import type { Asset } from '../../types/domain'
import { assetRepository } from '../../services/assets/assetRepository'

export function useAssets() {
  const [assets, setAssets] = useState<Asset[]>([])
  const [filter, setFilter] = useState<'全部' | '最近生成' | '收藏'>('全部')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const refresh = useCallback(async () => { setLoading(true); setAssets(await assetRepository.list()); setLoading(false) }, [])
  useEffect(() => {
    let mounted = true
    void assetRepository.list().then(next => {
      if (mounted) { setAssets(next); setLoading(false) }
    })
    return () => { mounted = false }
  }, [])
  const visibleAssets = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    return assets.filter(asset => (filter !== '收藏' || asset.favorite) && (!normalized || `${asset.id} ${asset.kind} ${asset.mimeType}`.toLowerCase().includes(normalized)))
  }, [assets, filter, query])
  const remove = useCallback(async (id: string) => { await assetRepository.remove(id); await refresh() }, [refresh])
  const toggleFavorite = useCallback(async (id: string) => { await assetRepository.toggleFavorite(id); await refresh() }, [refresh])
  const openLocation = useCallback((id: string) => assetRepository.openLocation(id), [])
  return { assets, visibleAssets, filter, setFilter, query, setQuery, loading, refresh, remove, toggleFavorite, openLocation }
}
