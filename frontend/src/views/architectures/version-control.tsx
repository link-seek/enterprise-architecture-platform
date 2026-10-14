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
  const { data, loading } = useQuery<any>(query, {
    variables: { spaceId, logicalId },
    skip: !logicalId || !spaceId,
  })

  const versions = (data?.[dataKey] || []) as { id: string; name: string; description: string; businessVersion: string; status: string; createdAt: string; updatedAt: string }[]

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{title ?? '版本历史'}</DialogTitle>
        </DialogHeader>
        <div className="py-4">
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
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
                      <Badge variant={v.status === 'archived' ? 'destructive' : v.status === 'deprecated' ? 'secondary' : v.status === 'active' ? 'default' : 'outline'}>
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

// ============================================================================
// Archive Button (inline)
// ============================================================================

export function ArchiveButton({ id, onArchived, spaceId }: { id: string; onArchived?: () => void; spaceId: string }) {
  const [archiveMut] = useMutation(ARCHIVE_VALUE_STREAM, {
    refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
  })
  const [loading, setLoading] = useState(false)

  async function handleArchive() {
    setLoading(true)
    try {
      await archiveMut({ variables: { id } })
      onArchived?.()
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
