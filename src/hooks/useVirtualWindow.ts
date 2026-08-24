import { useLayoutEffect, useMemo, useState } from 'react'

export function useVirtualWindow<T>(items: T[], itemHeight: number, overscan = 6, viewportId = 'virtual-window') {
  const [scrollTop, setScrollTop] = useState(0)
  const [viewportHeight, setViewportHeight] = useState(560)
  useLayoutEffect(() => {
    const element = document.getElementById(viewportId) as HTMLDivElement | null
    if (!element) return
    const measure = () => setViewportHeight(element.clientHeight || 560)
    const onScroll = () => setScrollTop(element.scrollTop)
    measure()
    element.addEventListener('scroll', onScroll, { passive: true })
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => { element.removeEventListener('scroll', onScroll); observer.disconnect() }
  }, [viewportId])
  const range = useMemo(() => {
    const first = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan)
    const last = Math.min(items.length, first + Math.ceil(viewportHeight / itemHeight) + overscan * 2)
    return { first, last }
  }, [itemHeight, items.length, overscan, scrollTop, viewportHeight])
  return { visibleItems: items.slice(range.first, range.last), topSpacer: range.first * itemHeight, bottomSpacer: Math.max(0, (items.length - range.last) * itemHeight) }
}
