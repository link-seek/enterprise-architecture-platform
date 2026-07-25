import { useState } from 'react'
import { useQuery, useMutation, useLazyQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Trash2, Loader2 } from 'lucide-react'
import { GET_SPACE_MEMBERS, ADD_SPACE_MEMBER, REMOVE_SPACE_MEMBER } from '@/api/spaces'
import type { SpaceMember } from '@/api/spaces'

const SPACE_USER_BY_EMAIL = gql`
  query SpaceUserByEmail($spaceId: String!, $email: String!) {
    spaceUserByEmail(spaceId: $spaceId, email: $email) {
      id
      name
    }
  }
`

interface UserResult {
  spaceUserByEmail: { id: string; name: string } | null
}

export function SpaceMembersDialog({ spaceId, open, onOpenChange }: {
  spaceId: string
  open: boolean
  onOpenChange: (v: boolean) => void
}) {
  const [email, setEmail] = useState('')
  const [role, setRole] = useState<'editor' | 'owner'>('editor')
  const [error, setError] = useState<string | null>(null)

  const { data, loading } = useQuery<{ spaceMembersBySpace: SpaceMember[] }>(
    GET_SPACE_MEMBERS,
    { variables: { spaceId }, skip: !open },
  )

  const [addMember, { loading: adding }] = useMutation(ADD_SPACE_MEMBER, {
    refetchQueries: [{ query: GET_SPACE_MEMBERS, variables: { spaceId } }],
    onCompleted: () => {
      setEmail('')
      setError(null)
    },
    onError: (e) => setError(e.message),
  })

  const [removeMember] = useMutation(REMOVE_SPACE_MEMBER, {
    refetchQueries: [{ query: GET_SPACE_MEMBERS, variables: { spaceId } }],
    onError: (e) => setError(e.message),
  })

  // Look up user by email, then add as member.
  const [lookupUser] = useLazyLookupUser(spaceId)

  async function handleAdd() {
    setError(null)
    const user = await lookupUser(email)
    if (!user) {
      setError('未找到该邮箱对应的用户')
      return
    }
    addMember({ variables: { spaceId, userId: user.id, role } })
  }

  const members = data?.spaceMembersBySpace ?? []

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>空间成员</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>用户</TableHead>
                <TableHead>角色</TableHead>
                <TableHead className="w-12"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading && (
                <TableRow><TableCell colSpan={3} className="text-center text-muted-foreground">加载中...</TableCell></TableRow>
              )}
              {!loading && members.length === 0 && (
                <TableRow><TableCell colSpan={3} className="text-center text-muted-foreground">暂无成员</TableCell></TableRow>
              )}
              {members.map((m) => (
                <TableRow key={m.userId}>
                  <TableCell>{m.name}</TableCell>
                  <TableCell>
                    <Badge variant={m.role === 'owner' ? 'default' : 'secondary'}>
                      {m.role === 'owner' ? '拥有者' : '编辑者'}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    {m.role !== 'owner' && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => removeMember({ variables: { spaceId, userId: m.userId } })}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>

          <div className="space-y-2 border-t pt-4">
            <Label>添加成员（按邮箱）</Label>
            <div className="flex gap-2">
              <Input
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="user@example.com"
              />
              <select
                className="h-9 rounded-md border bg-background px-2 text-sm"
                value={role}
                onChange={(e) => setRole(e.target.value as 'editor' | 'owner')}
              >
                <option value="editor">编辑者</option>
                <option value="owner">拥有者</option>
              </select>
              <Button onClick={handleAdd} disabled={adding || !email.trim()}>
                {adding && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                添加
              </Button>
            </div>
            {error && <p className="text-sm text-destructive">{error}</p>}
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// Local hook to look up a user by email via the space-scoped GraphQL query.
// This uses `spaceUserByEmail` (authorized by space membership) instead of the
// `users` entity (which requires global admin), so non-admin space owners can
// add members.
function useLazyLookupUser(spaceId: string) {
  const [lookup] = useLazyQuery<UserResult>(SPACE_USER_BY_EMAIL)
  async function lookupUser(email: string) {
    const res = await lookup({ variables: { spaceId, email } })
    return res.data?.spaceUserByEmail ?? null
  }
  return [lookupUser] as const
}