import { useState, useEffect } from 'react'
import type { ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'

// 与后端 `bump_minor` 一致的版本号建议：X.Y.Z → X.(Y+1).0，用户可改。
export function suggestNextVersion(current?: string | null): string {
  const m = (current ?? '').match(/(\d+)\.(\d+)\.(\d+)/)
  if (!m) return '1.1.0'
  return `${m[1]}.${parseInt(m[2], 10) + 1}.0`
}

// 统一发布仪式（discussions#752 统一发布策略 v1）：当前版本 → 新版本号 →
// 影响范围预览 → 确认发布 → 成功回执。价值流与业务流程共用同一骨架，
// 差异只在调用方传入的影响列表与提交函数。
export interface PublishImpactItem {
  id: string
  name: string
  detail?: string
}

export function VersionPublishDialog({ open, onOpenChange, title, currentVersion, suggestedVersion, impactIntro, impactItems, impactLoading, impactEmptyText, confirmLabel, successTitle, onConfirm }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  title: string
  currentVersion?: string | null
  suggestedVersion: string
  impactIntro: ReactNode
  impactItems: PublishImpactItem[]
  impactLoading?: boolean
  impactEmptyText: string
  confirmLabel?: string
  successTitle?: string
  onConfirm: (newVersion: string) => Promise<PublishImpactItem[]>
}) {
  const [newVersion, setNewVersion] = useState(suggestedVersion)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState<PublishImpactItem[] | null>(null)

  useEffect(() => {
    if (open) {
      setNewVersion(suggestedVersion)
      setError(null)
      setDone(null)
    }
  }, [open, suggestedVersion])

  async function handleConfirm() {
    setLoading(true)
    setError(null)
    try {
      const moved = await onConfirm(newVersion)
      setDone(moved)
    } catch (err) {
      setError(err instanceof Error ? err.message : '发布失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{title}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {done === null ? (
            <>
              <p className="text-muted-foreground">
                当前版本 <Badge variant="secondary" className="font-mono">{currentVersion ?? '-'}</Badge>{' '}
                将进入 <Badge variant="secondary">deprecated</Badge> 兼容期，
                新版本 <Badge>{newVersion || '?'}</Badge> 将成为 active，关联同步迁移。
              </p>
              <div className="space-y-2">
                <Label htmlFor="publish-new-version">新版本号</Label>
                <Input id="publish-new-version" value={newVersion} onChange={e => setNewVersion(e.target.value)} placeholder="2.0.0" />
              </div>
              <div className="space-y-2">
                <p className="font-medium">{impactIntro}</p>
                {impactLoading ? (
                  <div className="text-center py-4 text-muted-foreground">加载影响范围...</div>
                ) : impactItems.length === 0 ? (
                  <p className="text-muted-foreground">{impactEmptyText}</p>
                ) : (
                  <ul className="space-y-1">
                    {impactItems.map(item => (
                      <li key={item.id} className="flex items-center gap-2">
                        <span>{item.name}</span>
                        {item.detail && <span className="font-mono text-xs text-muted-foreground">{item.detail}</span>}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </>
          ) : (
            <>
              <p className="font-medium">{successTitle ?? '发布成功，关联已迁移到新版:'}</p>
              {done.length === 0 ? (
                <p className="text-muted-foreground">无受影响的关联。</p>
              ) : (
                <ul className="space-y-1">
                  {done.map(item => (
                    <li key={item.id} className="flex items-center gap-2">
                      <span>{item.name}</span>
                      {item.detail && <span className="font-mono text-xs text-muted-foreground">{item.detail}</span>}
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{done === null ? '取消' : '关闭'}</Button>
          {done === null && (
            <Button onClick={handleConfirm} disabled={loading || !newVersion || impactLoading}>
              {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : (confirmLabel ?? '确认发布')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
