import { useQuery, useMutation } from '@apollo/client/react'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'
import { GET_SPACE_MEMBERS } from '@/api/spaces'
import type { DocumentNode } from 'graphql'

interface SpaceMember {
  userId: string
  name: string
  role: string
}

/**
 * Generic "transfer ownership" dialog. The caller supplies the entity id,
 * the transfer mutation (which must accept `$id: String!` and
 * `$newOwnerId: String!`) and the refetch queries. The target must be a
 * member of the same space (enforced by the backend; the UI only lists
 * space members to pick from).
 */
export function TransferOwnershipDialog({ open, onOpenChange, entityId, spaceId, entityLabel, mutation, refetchQueries }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  entityId: string | null
  spaceId: string | undefined
  entityLabel: string
  mutation: DocumentNode
  refetchQueries?: { query: DocumentNode; variables?: Record<string, unknown> }[]
}) {
  const [newOwnerId, setNewOwnerId] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const { data } = useQuery<{ spaceMembersBySpace: SpaceMember[] }>(GET_SPACE_MEMBERS, {
    variables: { spaceId },
    skip: !spaceId || !open,
  })
  const [transferMut] = useMutation(mutation)

  const members = data?.spaceMembersBySpace ?? []

  useEffect(() => {
    if (open) {
      setError(null)
      setLoading(false)
      setNewOwnerId('')
    }
  }, [open])

  async function handleTransfer() {
    if (!entityId || !newOwnerId) return
    setLoading(true)
    setError(null)
    try {
      await transferMut({
        variables: { id: entityId, newOwnerId },
        refetchQueries,
      })
      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '转移所有权失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>转移{entityLabel}所有权</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <p className="text-sm text-muted-foreground">
            转移后仅新所有者（或管理员）可修改该{entityLabel}。新所有者必须是本空间的成员。
          </p>
          <div className="space-y-2">
            <Label htmlFor="transfer-new-owner">新所有者</Label>
            <select
              id="transfer-new-owner"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={newOwnerId}
              onChange={e => setNewOwnerId(e.target.value)}
            >
              <option value="">请选择成员</option>
              {members.map((m: SpaceMember) => (
                <option key={m.userId} value={m.userId}>
                  {m.name}（{m.role}）
                </option>
              ))}
            </select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleTransfer} disabled={loading || !newOwnerId}>
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '转移'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
