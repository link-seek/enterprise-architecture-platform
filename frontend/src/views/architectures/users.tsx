import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useState } from 'react'
import { createUser, updateRole } from '@/api/auth'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog'
import { Loader2, UserPlus } from 'lucide-react'

const GET_USERS = gql`
  query GetUsers {
    users {
      nodes {
        id
        email
        name
        role
        status
      }
    }
  }
`

interface User {
  id: string
  email: string
  name: string
  role: string
  status: string
}

const ROLE_OPTIONS = ['admin', 'architect', 'viewer'] as const

export default function UsersPage() {
  const { data, loading, refetch } = useQuery(GET_USERS)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [form, setForm] = useState({ email: '', name: '', password: '', role: 'viewer' })
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [roleUpdating, setRoleUpdating] = useState<string | null>(null)
  const [confirmRole, setConfirmRole] = useState<{ user: User; newRole: string } | null>(null)

  const users: User[] = data?.users?.nodes || []

  async function handleCreate() {
    setSubmitting(true)
    setError(null)
    try {
      await createUser(form.email, form.name, form.password, form.role)
      await refetch()
      setDialogOpen(false)
      setForm({ email: '', name: '', password: '', role: 'viewer' })
    } catch (err) {
      setError(err instanceof Error ? err.message : '创建用户失败')
    } finally {
      setSubmitting(false)
    }
  }

  async function handleRoleChange(userId: string, newRole: string) {
    setRoleUpdating(userId)
    setError(null)
    try {
      await updateRole(userId, newRole)
      await refetch()
      setConfirmRole(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : '修改角色失败')
    } finally {
      setRoleUpdating(null)
    }
  }

  function onRoleSelect(user: User, newRole: string) {
    if (newRole === user.role) return
    setConfirmRole({ user, newRole })
  }

  const isDangerousRoleChange =
    confirmRole !== null &&
    confirmRole.user.role === 'admin' &&
    confirmRole.newRole !== 'admin'

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">用户管理</h1>
        <Button onClick={() => setDialogOpen(true)}>
          <UserPlus className="h-4 w-4 mr-2" />
          添加账户
        </Button>
      </div>

      {loading ? (
        <div className="text-center py-8 text-muted-foreground">加载中...</div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>姓名</TableHead>
              <TableHead>邮箱</TableHead>
              <TableHead>角色</TableHead>
              <TableHead>状态</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {users.map((u) => (
              <TableRow key={u.id}>
                <TableCell className="font-medium">{u.name}</TableCell>
                <TableCell>{u.email}</TableCell>
                <TableCell>
                  <select
                    value={u.role}
                    disabled={roleUpdating === u.id}
                    onChange={(e) => onRoleSelect(u, e.target.value)}
                    className="border rounded px-2 py-1 text-sm bg-background"
                  >
                    {ROLE_OPTIONS.map((r) => (
                      <option key={r} value={r}>
                        {r}
                      </option>
                    ))}
                  </select>
                </TableCell>
                <TableCell>
                  <Badge variant={u.status === 'active' ? 'default' : 'destructive'}>
                    {u.status}
                  </Badge>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>添加账户</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {error && (
              <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                {error}
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="new-name">姓名</Label>
              <Input
                id="new-name"
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="张三"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="new-email">邮箱</Label>
              <Input
                id="new-email"
                type="email"
                value={form.email}
                onChange={(e) => setForm({ ...form, email: e.target.value })}
                placeholder="user@example.com"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="new-password">密码</Label>
              <Input
                id="new-password"
                type="password"
                value={form.password}
                onChange={(e) => setForm({ ...form, password: e.target.value })}
                placeholder="至少 8 位"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="new-role">角色</Label>
              <select
                id="new-role"
                value={form.role}
                onChange={(e) => setForm({ ...form, role: e.target.value })}
                className="w-full border rounded px-3 py-2 text-sm bg-background"
              >
                {ROLE_OPTIONS.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              取消
            </Button>
            <Button
              onClick={handleCreate}
              disabled={submitting || !form.email || !form.name || !form.password}
            >
              {submitting ? <Loader2 className="h-4 w-4 animate-spin" /> : '创建'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={confirmRole !== null}
        onOpenChange={(open) => {
          if (!open && roleUpdating === null) setConfirmRole(null)
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认修改角色</DialogTitle>
            <DialogDescription>
              {isDangerousRoleChange
                ? `即将把 ${confirmRole?.user.name} 的角色从 admin 降级为 ${confirmRole?.newRole}，该用户将失去管理员权限。`
                : `即将把 ${confirmRole?.user.name} 的角色修改为 ${confirmRole?.newRole}。`}
            </DialogDescription>
          </DialogHeader>
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setConfirmRole(null)}
              disabled={roleUpdating !== null}
            >
              取消
            </Button>
            <Button
              variant={isDangerousRoleChange ? 'destructive' : 'default'}
              disabled={roleUpdating !== null}
              onClick={() => {
                if (!confirmRole) return
                handleRoleChange(confirmRole.user.id, confirmRole.newRole)
              }}
            >
              {roleUpdating ? <Loader2 className="h-4 w-4 animate-spin" /> : '确认'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}