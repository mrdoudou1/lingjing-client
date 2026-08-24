import { useLayoutEffect, useMemo, useState, type ReactNode } from 'react'
import { useVirtualWindow } from '../../hooks/useVirtualWindow'

type VirtualGalleryProps<T> = {
  items: T[]
  renderItem: (item: T, index: number) => ReactNode
  rowHeight?: number
}

export function VirtualGallery<T>({ items, renderItem, rowHeight = 260 }: VirtualGalleryProps<T>) {
  const [columns, setColumns] = useState(5)
  const rows = useMemo(() => {
    const grouped: T[][] = []
    for (let index = 0; index < items.length; index += columns) grouped.push(items.slice(index, index + columns))
    return grouped
  }, [columns, items])
  const virtual = useVirtualWindow(rows, rowHeight, 2, 'assets-virtual-grid')

  useLayoutEffect(() => {
    const element = document.getElementById('assets-virtual-grid')
    if (!element) return
    const updateColumns = () => {
      const width = element.clientWidth
      setColumns(width < 520 ? 2 : width < 900 ? 4 : 5)
    }
    updateColumns()
    const observer = new ResizeObserver(updateColumns)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  const firstRow = Math.floor(virtual.topSpacer / rowHeight)
  return <div id="assets-virtual-grid" className="gallery-grid virtual-gallery"><div style={{ height: virtual.topSpacer }} aria-hidden="true" />{virtual.visibleItems.map((row, rowIndex) => <div className="virtual-gallery-row" key={`row-${firstRow + rowIndex}`} style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, minHeight: rowHeight }}>{row.map((item, index) => renderItem(item, (firstRow + rowIndex) * columns + index))}</div>)}<div style={{ height: virtual.bottomSpacer }} aria-hidden="true" /></div>
}
