import { useQuery } from '@apollo/client/react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Link, useParams } from 'react-router-dom'
import { Plus, Pencil, Trash2, History, GitBranch, MoreVertical } from 'lucide-react'
import { useState } from 'react'
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
  valueStreams?: { nodes: ValueStream[]; paginationInfo?: { total: number } }
}

type BadgeVariant = NonNullable<BadgeProps['variant']>

function statusColor(status: string): BadgeVariant {
  switch (status) {
    case 'active': return 'default'
    case 'archived': return 'destructive'
    default: return 'outline'
  }
}

function ValueStreamList({ nodes, canEdit, isMobile, detailBase, spaceId, onEdit, onDelete, onVersion, onHistory }: {
  nodes: ValueStream[]
  canEdit: boolean
  isMobile: boolean
  detailBase: string
  spaceId?: string
  onEdit: (vs: ValueStream) => void
  onDelete: (vs: ValueStream) => void
  onVersion: (vs: ValueStream) => void
  onHistory: (vs: ValueStream) => void
}) {
  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((vs) => (
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
                <Button variant="outline" size="sm">查看</Button>
              </Link>
              {canEdit && (
                <div className="flex items-center gap-1">
                  {vs.status === 'active' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="sm" className="h-9 w-9 p-0" aria-label="更多操作">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => onEdit(vs)}>
                        <Pencil className="h-4 w-4 mr-2" />编辑
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => onVersion(vs)}>
                        <GitBranch className="h-4 w-4 mr-2" />新版本
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => onHistory(vs)}>
                        <History className="h-4 w-4 mr-2" />历史
                      </DropdownMenuItem>
                      <DropdownMenuItem className="text-destructive" onClick={() => onDelete(vs)}>
                        <Trash2 className="h-4 w-4 mr-2" />删除
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              )}
            </div>
          </div>
        ))}
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
        {nodes.map((vs) => (
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
                    <Button variant="ghost" size="sm" onClick={() => onEdit(vs)}>
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => onVersion(vs)}>
                      <GitBranch className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => onHistory(vs)}>
                      <History className="h-3.5 w-3.5" />
                    </Button>
                    {vs.status === 'active' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                    <Button variant="ghost" size="sm" onClick={() => onDelete(vs)}>
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </>
                )}
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

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
  })

  const detailBase = spaceId
    ? `/spaces/${spaceId}/architectures/value-streams`
    : '/architectures/value-streams'

  const handleEdit = (vs: ValueStream) => { setEditing(vs); setDialogOpen(true) }
  const handleDelete = (vs: ValueStream) => setDeleting(vs)
  const handleVersion = (vs: ValueStream) => { setVersionItem(vs); setVersionOpen(true) }
  const handleHistory = (vs: ValueStream) => { setHistoryLogicalId(vs.logicalId); setHistoryOpen(true) }

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
          {error && <div className="text-center py-8 text-destructive">加载失败: {error.message}</div>}
          {data && (
            <>
              <ValueStreamList
                nodes={data.valueStreams?.nodes ?? []}
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
                <p className="text-sm text-muted-foreground">共 {data.valueStreams?.paginationInfo?.total ?? 0} 条</p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <ValueStreamCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ValueStreamDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <VersionHistoryDialog open={historyOpen} onOpenChange={setHistoryOpen} logicalId={historyLogicalId} />
      <CreateVersionDialog open={versionOpen} onOpenChange={setVersionOpen} currentItem={versionItem} spaceId={spaceId} />
    </div>
  )
}
