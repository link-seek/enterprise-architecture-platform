import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Link, useParams } from 'react-router-dom'
import { Plus, Pencil, Trash2, History, GitBranch } from 'lucide-react'
import { useState } from 'react'
import { ValueStreamCrudDialog, ValueStreamDeleteDialog } from './crud'
import { VersionHistoryDialog, CreateVersionDialog, ArchiveButton, GET_VALUE_STREAMS } from './version-control'
import { useSpaceMembership } from '@/hooks/use-space-membership'

interface ValueStream {
  id: string
  name: string
  description: string
  businessVersion: string
  status: string
  importance: string
  logicalId: string
}

export default function ValueStreams() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<ValueStream | null>(null)
  const [deleting, setDeleting] = useState<ValueStream | null>(null)
  const [historyOpen, setHistoryOpen] = useState(false)
  const [historyLogicalId, setHistoryLogicalId] = useState<string | null>(null)
  const [versionOpen, setVersionOpen] = useState(false)
  const [versionItem, setVersionItem] = useState<ValueStream | null>(null)

  const { data, loading, error } = useQuery(GET_VALUE_STREAMS, {
    variables: { spaceId },
  })

  const statusColor = (status: string) => {
    switch (status) {
      case 'active': return 'default'
      case 'archived': return 'destructive'
      default: return 'outline'
    }
  }

  const detailBase = spaceId
    ? `/spaces/${spaceId}/architectures/value-streams`
    : '/architectures/value-streams'

  return (
    <div className="p-6 space-y-4">
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
                  {data.valueStreams.nodes.map((vs: ValueStream) => (
                    <TableRow key={vs.id}>
                      <TableCell className="font-medium">
                        {vs.name}
                        <span className="ml-2 text-xs text-muted-foreground">{vs.description}</span>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary" className="font-mono">{vs.businessVersion}</Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant={statusColor(vs.status) as any}>{vs.status}</Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          <Link to={`${detailBase}/${vs.id}`}>
                            <Button variant="ghost" size="sm">查看</Button>
                          </Link>
                          {canEdit && (
                            <>
                              <Button variant="ghost" size="sm" onClick={() => { setEditing(vs); setDialogOpen(true) }}>
                                <Pencil className="h-3.5 w-3.5" />
                              </Button>
                              <Button variant="ghost" size="sm" onClick={() => { setVersionItem(vs); setVersionOpen(true) }}>
                                <GitBranch className="h-3.5 w-3.5" />
                              </Button>
                              <Button variant="ghost" size="sm" onClick={() => { setHistoryLogicalId(vs.logicalId); setHistoryOpen(true) }}>
                                <History className="h-3.5 w-3.5" />
                              </Button>
                              {vs.status === 'active' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                              <Button variant="ghost" size="sm" onClick={() => setDeleting(vs)}>
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
              <div className="flex items-center justify-between mt-4">
                <p className="text-sm text-muted-foreground">共 {data.valueStreams.paginationInfo.total} 条</p>
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
