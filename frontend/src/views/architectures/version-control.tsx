import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import type { DocumentNode } from '@apollo/client'
import { useState } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Loader2, Archive } from 'lucide-react'

// ============================================================================
// Version Control Queries & Mutations
// ============================================================================

const GET_VALUE_STREAM_VERSIONS = gql`
  query GetValueStreamVersions($spaceId: String!, $logicalId: String!) {
    valueStreamsBySpaceAndLogicalId(spaceId: $spaceId, logicalId: $logicalId) {
      id name description businessVersion status ownerId triggeringEvent endDeliverable stakeholders performanceMetrics createdAt updatedAt
    }
  }
`

const ARCHIVE_VALUE_STREAM = gql`
  mutation ValueStreamArchive($id: String!) {
    valueStreamArchive(id: $id)
  }
`

export { ARCHIVE_VALUE_STREAM }

export const ARCHIVE_PROCESS = gql`
  mutation ArchiveProcess($id: String!) {
    processArchive(id: $id)
  }
`

export const ARCHIVE_CAPABILITY = gql`
  mutation ArchiveCapability($id: String!) {
    capabilityArchive(id: $id)
  }
`

const GET_VALUE_STREAMS = gql`
  query GetValueStreamsForVersion($spaceId: String!) {
    valueStreamsBySpace(spaceId: $spaceId) {
      id name description businessVersion status importance logicalId spaceId ownerId triggeringEvent endDeliverable stakeholders performanceMetrics
    }
  }
`

// ============================================================================
// Version History Dialog (shared by value streams and business processes:
// unified publish policy, one viewer)
// ============================================================================

export function VersionHistoryDialog({ open, onOpenChange, spaceId, logicalId, title, query, dataKey }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  spaceId: string
  logicalId: string | null
  title?: string
  query: DocumentNode
  dataKey: string
}) {
  const { data, loading, error } = useQuery<any>(query, {
    variables: { spaceId, logicalId },
    skip: !logicalId || !spaceId,
  })

  const raw = data?.[dataKey]
  const versions = (Array.isArray(raw) ? raw : []) as { id: string; name: string; description: string; businessVersion: string; status: string; createdAt: string; updatedAt: string }[]

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{title ?? '版本历史'}</DialogTitle>
        </DialogHeader>
        <div className="py-4">
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {!loading && error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">版本历史加载失败</div>}
          {!loading && !error && versions.length === 0 && <div className="text-center py-8 text-muted-foreground">暂无版本记录</div>}
          {versions.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>版本</TableHead>
                  <TableHead>名称</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead>创建时间</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {versions.map((v: any) => (
                  <TableRow key={v.id}>
                    <TableCell className="font-mono">{v.businessVersion}</TableCell>
                    <TableCell>{v.name}</TableCell>
                    <TableCell>
                      <Badge variant={historyStatusVariant(v.status)}>
                        {v.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {new Date(v.createdAt).toLocaleString('zh-CN')}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

const HISTORY_STATUS_VARIANTS = {
  active: 'default',
  deprecated: 'secondary',
  archived: 'destructive',
} as const

function historyStatusVariant(status: string): 'default' | 'destructive' | 'secondary' | 'outline' {
  return (HISTORY_STATUS_VARIANTS as Record<string, 'default' | 'destructive' | 'secondary' | 'outline'>)[status] ?? 'outline'
}

// ============================================================================
// Archive Button (shared, inline, no confirm dialog).
// The mutation + list refetch are parameterized so value streams, processes
// and capabilities share one component instead of three copies.
// ============================================================================

export function ArchiveButton({ id, spaceId, mutation, refetchQuery }: {
  id: string
  spaceId: string
  mutation: DocumentNode
  refetchQuery: DocumentNode
}) {
  const [archiveMut] = useMutation(mutation, {
    refetchQueries: [{ query: refetchQuery, variables: { spaceId } }],
  })
  const [loading, setLoading] = useState(false)

  async function handleArchive() {
    setLoading(true)
    try {
      await archiveMut({ variables: { id } })
    } catch (err) {
      console.error('Archive failed:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <Button variant="ghost" size="sm" aria-label="归档" title="归档" onClick={handleArchive} disabled={loading}>
      {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Archive className="h-3.5 w-3.5" />}
    </Button>
  )
}

export { GET_VALUE_STREAMS, GET_VALUE_STREAM_VERSIONS }
