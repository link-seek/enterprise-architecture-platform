import { useQuery } from '@apollo/client/react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Link, useParams } from 'react-router-dom'
import { Plus, Pencil, Trash2, History, GitBranch, MoreVertical, type LucideIcon } from 'lucide-react'
import { useState, useCallback, memo } from 'react'
import { ValueStreamCrudDialog, ValueStreamDeleteDialog } from './crud'
import { VersionHistoryDialog, CreateVersionDialog, ArchiveButton, GET_VALUE_STREAMS } from './version-control'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'

interface ValueStream {
  id: string
  name: string
  description: string
  businessVersion: string
  status: string
  importance: string
  logicalId: string
}

interface ValueStreamsQuery {
  valueStreamsBySpace?: ValueStream[]
}

const EMPTY_VALUE_STREAMS: ValueStream[] = []

type BadgeVariant = NonNullable<BadgeProps['variant']>

function statusColor(status: string): BadgeVariant {
  switch (status) {
    case 'active': return 'default'
    case 'archived': return 'destructive'
    default: return 'outline'
  }
}

interface ValueStreamAction {
  icon: LucideIcon
  label: string
  onClick: () => void
  destructive?: boolean
}

function buildActions(vs: ValueStream, onEdit: (vs: ValueStream) => void, onVersion: (vs: ValueStream) => void, onHistory: (vs: ValueStream) => void, onDelete: (vs: ValueStream) => void): ValueStreamAction[] {
  return [
    { icon: Pencil, label: '编辑', onClick: () => onEdit(vs) },
    { icon: GitBranch, label: '新版本', onClick: () => onVersion(vs) },
    { icon: History, label: '历史', onClick: () => onHistory(vs) },
    { icon: Trash2, label: '删除', onClick: () => onDelete(vs), destructive: true },
  ]
}

const ValueStreamList = memo(function ValueStreamList({ nodes, canEdit, isMobile, detailBase, spaceId, onEdit, onDelete, onVersion, onHistory }: {
  nodes: ValueStream[]
  canEdit: boolean
  isMobile: boolean
  detailBase: string
  spaceId: string
  onEdit: (vs: ValueStream) => void
  onDelete: (vs: ValueStream) => void
  onVersion: (vs: ValueStream) => void
  onHistory: (vs: ValueStream) => void
}) {
  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((vs) => {
          const actions = buildActions(vs, onEdit, onVersion, onHistory, onDelete)
          return (
          <div key={vs.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="font-medium truncate" title={vs.name}>{vs.name}</p>
                {vs.description && (
                  <p className="text-xs text-muted-foreground truncate" title={vs.description}>{vs.description}</p>
                )}
              </div>
              <div className="flex gap-1 shrink-0">
                <Badge variant="secondary" className="font-mono">{vs.businessVersion}</Badge>
                <Badge variant={statusColor(vs.status)}>{vs.status}</Badge>
              </div>
            </div>
            <div className="flex items-center justify-between pt-1">
              <Link to={`${detailBase}/${vs.id}`}>
                {/* 移动端使用 outline 增强触摸可见性，桌面端使用 ghost 保持简洁 */}
                <Button variant="outline" size="sm">查看</Button>
              </Link>
              {canEdit && (
                <div className="flex items-center gap-1">
                  {vs.status === 'active' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="sm" className="h-11 w-11 p-0" aria-label="更多操作">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      {actions.map((action) => {
                        const Icon = action.icon
                        return (
                          <DropdownMenuItem key={action.label} className={action.destructive ? 'text-destructive' : undefined} onClick={action.onClick}>
                            <Icon className="h-4 w-4 mr-2" />{action.label}
                          </DropdownMenuItem>
                        )
                      })}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </div>
          </div>
          )
        })}
      </div>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>版本</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((vs) => {
          const actions = buildActions(vs, onEdit, onVersion, onHistory, onDelete)
          return (
          <TableRow key={vs.id}>
            <TableCell className="font-medium">
              {vs.name}
              <span className="ml-2 text-xs text-muted-foreground">{vs.description}</span>
            </TableCell>
            <TableCell>
              <Badge variant="secondary" className="font-mono">{vs.businessVersion}</Badge>
            </TableCell>
            <TableCell>
              <Badge variant={statusColor(vs.status)}>{vs.status}</Badge>
            </TableCell>
            <TableCell>
              <div className="flex gap-1">
                <Link to={`${detailBase}/${vs.id}`}>
                  <Button variant="ghost" size="sm">查看</Button>
                </Link>
                {canEdit && (
                  <>
                    {actions.map((action) => {
                      const Icon = action.icon
                      return (
                        <Button key={action.label} variant="ghost" size="sm" aria-label={action.label} onClick={action.onClick}>
                          <Icon className={`h-3.5 w-3.5${action.destructive ? ' text-destructive' : ''}`} />
                        </Button>
                      )
                    })}
                    {vs.status === 'active' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                  </>
                )}
              </div>
            </TableCell>
          </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
})

export default function ValueStreams() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<ValueStream | null>(null)
  const [deleting, setDeleting] = useState<ValueStream | null>(null)
  const [historyOpen, setHistoryOpen] = useState(false)
  const [historyLogicalId, setHistoryLogicalId] = useState<string | null>(null)
  const [versionOpen, setVersionOpen] = useState(false)
  const [versionItem, setVersionItem] = useState<ValueStream | null>(null)

  const { data, loading, error } = useQuery<ValueStreamsQuery>(GET_VALUE_STREAMS, {
    variables: { spaceId },
    skip: !spaceId,
  })

  const handleEdit = useCallback((vs: ValueStream) => { setEditing(vs); setDialogOpen(true) }, [])
  const handleDelete = useCallback((vs: ValueStream) => setDeleting(vs), [])
  const handleVersion = useCallback((vs: ValueStream) => { setVersionItem(vs); setVersionOpen(true) }, [])
  const handleHistory = useCallback((vs: ValueStream) => { setHistoryLogicalId(vs.logicalId); setHistoryOpen(true) }, [])

  if (!spaceId) {
    return (
      <div className="p-6">
        <div className="text-center py-8 text-destructive">缺少空间标识，无法加载价值流。</div>
      </div>
    )
  }

  const detailBase = `/spaces/${spaceId}/architectures/value-streams`

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">价值流</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />
            新建价值流
          </Button>
        )}
      </div>

      <Card>
        <CardHeader><CardTitle>价值流列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <>
              <ValueStreamList
                nodes={data.valueStreamsBySpace ?? EMPTY_VALUE_STREAMS}
                canEdit={canEdit}
                isMobile={isMobile}
                detailBase={detailBase}
                spaceId={spaceId}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onVersion={handleVersion}
                onHistory={handleHistory}
              />
              <div className="flex items-center justify-between mt-4">
                <p className="text-sm text-muted-foreground">共 {data.valueStreamsBySpace?.length ?? 0} 条</p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <ValueStreamCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ValueStreamDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <VersionHistoryDialog open={historyOpen} onOpenChange={setHistoryOpen} spaceId={spaceId} logicalId={historyLogicalId} />
      <CreateVersionDialog open={versionOpen} onOpenChange={setVersionOpen} currentItem={versionItem} spaceId={spaceId} />
    </div>
  )
}
