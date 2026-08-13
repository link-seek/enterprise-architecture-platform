import { Link } from 'react-router-dom'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Loader2 } from 'lucide-react'

export interface CrossDomainItem {
  id: string
  name: string
}

// 跨域关联弹窗：展示对侧域实体列表，每一行可点击跳转到对侧页面。
export function CrossDomainDialog({ open, onOpenChange, title, items, loading, to }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  title: string
  items: CrossDomainItem[]
  loading: boolean
  to: string
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{title}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {loading ? (
            <div className="flex items-center justify-center gap-2 py-6 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />加载中...
            </div>
          ) : items.length === 0 ? (
            <div className="text-center py-6 text-muted-foreground">暂无数据</div>
          ) : (
            <ul className="space-y-2">
              {items.map((item) => (
                <li key={item.id}>
                  <Link
                    to={to}
                    onClick={() => onOpenChange(false)}
                    className="block rounded-md border px-3 py-2 font-medium transition-colors hover:border-primary hover:text-primary"
                  >
                    {item.name}
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
