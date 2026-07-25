import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { History, Loader2, Archive } from 'lucide-react'

// ============================================================================
// Version Control Queries & Mutations
// ============================================================================

const GET_VALUE_STREAM_VERSIONS = gql`
  query GetValueStreamVersions($logicalId: String!) {
    valueStreams(filters: { logicalId: { eq: $logicalId } }) {
      nodes { id name description businessVersion status createdAt updatedAt }
    }
  }
`

const ARCHIVE_VALUE_STREAM = gql`
  mutation ValueStreamArchive($id: String!) {
    valueStreamArchive(id: $id)
  }
`

const CREATE_VERSION = gql`
  mutation ValueStreamCreateVersion($currentId: String!, $newVersion: String!, $newName: String, $newDescription: String) {
    valueStreamCreateVersion(currentId: $currentId, newVersion: $newVersion, newName: $newName, newDescription: $newDescription) {
      id name description businessVersion status importance logicalId
    }
  }
`

const GET_VALUE_STREAMS = gql`
  query GetValueStreamsForVersion($spaceId: String) {
    valueStreams(filters: { spaceId: { eq: $spaceId } }) {
      nodes { id name description businessVersion status importance logicalId spaceId }
      paginationInfo { total }
    }
  }
`

// ============================================================================
// Version History Dialog
// ============================================================================

export function VersionHistoryDialog({ open, onOpenChange, logicalId }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  logicalId: string | null
}) {
  const { data, loading } = useQuery(GET_VALUE_STREAM_VERSIONS, {
    variables: { logicalId },
    skip: !logicalId,
  })

  const versions = data?.valueStreams?.nodes || []

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>版本历史</DialogTitle>
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
                      <Badge variant={v.status === 'active' ? 'default' : 'destructive'}>
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
// Create Version Dialog
// ============================================================================

export function CreateVersionDialog({ open, onOpenChange, currentItem, spaceId }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  currentItem: { id: string; logicalId: string; name: string; description: string; businessVersion: string; importance: string } | null
  spaceId?: string
}) {
  const [newVersion, setNewVersion] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createVersionMut] = useMutation(CREATE_VERSION)

  useEffect(() => {
    if (open && currentItem) {
      const match = currentItem.businessVersion.match(/v(\d+)\.(\d+)/)
      if (match) {
        setNewVersion(`v${parseInt(match[1]) + 1}.0`)
      } else {
        setNewVersion('v2.0')
      }
      setError(null)
    }
  }, [open, currentItem])

  async function handleCreateVersion() {
    if (!currentItem) return
    setLoading(true)
    setError(null)
    try {
      // Single atomic mutation: archives current + creates new version
      await createVersionMut({
        variables: {
          currentId: currentItem.id,
          newVersion,
          newName: currentItem.name,
          newDescription: currentItem.description,
        },
        refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
      })

      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建版本失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>新建版本</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <p className="text-sm text-muted-foreground">
            当前版本 <Badge variant="secondary">{currentItem?.businessVersion}</Badge> 将被归档，
            新版本 <Badge>{newVersion}</Badge> 将成为 active。
          </p>
          <div className="space-y-2">
            <Label>新版本号</Label>
            <Input value={newVersion} onChange={e => setNewVersion(e.target.value)} placeholder="v2.0" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleCreateVersion} disabled={loading || !newVersion}>
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '创建新版本'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// ============================================================================
// Archive Button (inline)
// ============================================================================

export function ArchiveButton({ id, onArchived, spaceId }: { id: string; onArchived?: () => void; spaceId?: string }) {
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
    <Button variant="ghost" size="sm" onClick={handleArchive} disabled={loading}>
      {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Archive className="h-3.5 w-3.5" />}
    </Button>
  )
}

export { GET_VALUE_STREAMS, GET_VALUE_STREAM_VERSIONS }
