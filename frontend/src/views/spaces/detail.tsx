import { useState, useCallback, useMemo } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { useQuery, useMutation } from '@apollo/client/react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Pencil, Archive, LogIn, ArrowLeft, Users, MoreVertical, X, Eye, EyeOff } from 'lucide-react'
import { GET_SPACE, ARCHIVE_SPACE, GET_SPACES, GET_SPACE_STATS, SET_SPACE_VISIBILITY } from '@/api/spaces'
import type { Space, SpaceStats, SpaceVisibility } from '@/api/spaces'
import { useAuthStore } from '@/stores/auth'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { SpaceEditDialog } from './crud'
import { SpaceMembersDialog } from './members'

function extractFriendlyError(e: { message?: string }): string {
  const msg = e.message ?? ''
  if (/network|fetch|timeout/i.test(msg)) return '网络错误，请稍后重试'
  if (/unauthorized|forbidden|401|403/i.test(msg)) return '权限不足，操作被拒绝'
  return '操作失败，请稍后重试'
}

export default function SpaceDetail() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const navigate = useNavigate()
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const { canEdit, role } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [editOpen, setEditOpen] = useState(false)
  const [membersOpen, setMembersOpen] = useState(false)
  const [visibilityError, setVisibilityError] = useState<string | null>(null)
  const [pendingVisibility, setPendingVisibility] = useState<SpaceVisibility | null>(null)
  const [confirmArchive, setConfirmArchive] = useState(false)
  const [archiveError, setArchiveError] = useState<string | null>(null)

  const { data, loading, error } = useQuery<{ spaceById: Space | null }>(GET_SPACE, {
    variables: { id: spaceId },
    skip: !spaceId,
  })
  const { data: stats } = useQuery<SpaceStats>(GET_SPACE_STATS, {
    variables: { spaceId },
    skip: !spaceId,
  })

  const [archive, { loading: archiveLoading }] = useMutation(ARCHIVE_SPACE, {
    refetchQueries: [{ query: GET_SPACES }],
    onCompleted: () => navigate('/spaces'),
    onError: (e) => { console.error('归档空间失败:', e); setArchiveError(extractFriendlyError(e)) },
  })

  const [setVisibility, { loading: visibilityLoading }] = useMutation(SET_SPACE_VISIBILITY, {
    refetchQueries: [{ query: GET_SPACE, variables: { id: spaceId } }],
    onError: (e) => { console.error('设置可见性失败:', e); setVisibilityError(extractFriendlyError(e)) },
    onCompleted: () => {
      setVisibilityError(null)
      setPendingVisibility(null)
    },
  })

  const space = data?.spaceById

  const clearAllErrors = useCallback(() => { setArchiveError(null); setVisibilityError(null) }, [])
  const handleEdit = useCallback(() => { clearAllErrors(); setEditOpen(true) }, [clearAllErrors])
  const handleMembers = useCallback(() => { clearAllErrors(); setMembersOpen(true) }, [clearAllErrors])
  const handleArchive = useCallback(() => { clearAllErrors(); setConfirmArchive(true) }, [clearAllErrors])
  const handleVisibility = useCallback(() => {
    if (!space) return
    clearAllErrors()
    setPendingVisibility(space.visibility === 'public' ? 'private' : 'public')
  }, [space, clearAllErrors])

  const statsItems = [
    { label: '价值流', value: stats?.valueStreamCountBySpace ?? 0, to: 'value-streams' },
    { label: '业务能力', value: stats?.businessCapabilityCountBySpace ?? 0, to: 'capabilities' },
    { label: '业务流程', value: stats?.businessProcessCountBySpace ?? 0, to: 'processes' },
  ]

  const visibleActions = useMemo(() => [
    { icon: Pencil, label: '编辑', onClick: handleEdit, visible: canEdit },
    { icon: Users, label: '成员', onClick: handleMembers, visible: role === 'owner' },
    { icon: space?.visibility === 'public' ? EyeOff : Eye, label: space?.visibility === 'public' ? '设为私有' : '设为公开', onClick: handleVisibility, visible: role === 'owner' },
    { icon: Archive, label: '归档', onClick: handleArchive, visible: role === 'owner' },
  ].filter((a) => a.visible), [canEdit, role, handleEdit, handleMembers, handleArchive, handleVisibility, space])

  if (loading) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">加载中...</div>
  if (error) return <div className="min-h-screen flex items-center justify-center text-destructive">加载失败: {error.message}</div>
  if (!space) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">空间不存在</div>

  return (
    <div className="min-h-screen bg-secondary flex flex-col">
      <header className="border-b bg-background">
        <div className="container mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <div className="flex items-center gap-3">
            <Link to="/spaces" className="flex items-center gap-2 text-muted-foreground hover:text-foreground">
              <ArrowLeft className="h-4 w-4" />
              空间
            </Link>
            <span className="text-lg font-semibold">{space.name}</span>
            {role && <Badge variant="secondary">{role === 'owner' ? '拥有者' : '编辑者'}</Badge>}
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            {canEdit && (
              isMobile ? (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="outline" size="sm" className="h-9 w-9 p-0" aria-label="更多操作">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    {visibleActions.map((action) => {
                      const Icon = action.icon
                      return (
                        <DropdownMenuItem key={action.label} onClick={action.onClick}>
                          <Icon className="h-4 w-4 mr-2" />{action.label}
                        </DropdownMenuItem>
                      )
                    })}
                  </DropdownMenuContent>
                </DropdownMenu>
              ) : (
                <>
                  {visibleActions.map((action) => {
                    const Icon = action.icon
                    return (
                      <Button key={action.label} variant="outline" size="sm" onClick={action.onClick} disabled={archiveLoading || visibilityLoading}>
                        <Icon className="h-4 w-4 mr-2" />
                        {action.label}
                      </Button>
                    )
                  })}
                </>
              )
            )}
            {!isAuthenticated && (
              <Button variant="outline" size="sm" onClick={() => navigate('/login')}>
                <LogIn className="h-4 w-4 mr-2" />
                登录以编辑
              </Button>
            )}
          </div>
        </div>
      </header>

      <main className="flex-1 container mx-auto max-w-6xl px-4 py-10">
        {archiveError && (
          <div role="alert" className="mb-4 flex items-start justify-between gap-3 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            <span>{archiveError}</span>
            <button
              type="button"
              onClick={() => setArchiveError(null)}
              aria-label="关闭错误提示"
              className="shrink-0 text-destructive hover:opacity-70"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        )}
        <p className="text-muted-foreground">{space.description || '暂无描述'}</p>

        <div className="mt-8 grid gap-6 md:grid-cols-3">
          {statsItems.map((item) => (
            <Link key={item.to} to={`/spaces/${space.id}/architectures/${item.to}`}>
              <Card className="h-full hover:shadow-md transition-shadow">
                <CardHeader>
                  <CardTitle>{item.label}</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-3xl font-bold">{item.value}</p>
                  <p className="mt-1 text-sm text-muted-foreground">点击查看详情</p>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </main>

      <SpaceEditDialog space={space} open={editOpen} onOpenChange={setEditOpen} />
      <SpaceMembersDialog spaceId={space.id} open={membersOpen} onOpenChange={setMembersOpen} />

      <ConfirmDialog
        open={pendingVisibility !== null}
        onOpenChange={(v) => {
          if (!v) {
            setPendingVisibility(null)
            setVisibilityError(null)
          }
        }}
        title={`设为${pendingVisibility === 'public' ? '公开' : '私有'}`}
        description={`确定将此空间设为${pendingVisibility === 'public' ? '公开' : '私有'}？`}
        confirmText="确定"
        loading={visibilityLoading}
        error={visibilityError}
        onConfirm={() => {
          if (pendingVisibility) {
            setVisibilityError(null)
            setVisibility({ variables: { id: space.id, visibility: pendingVisibility } })
          }
        }}
      />

      <ConfirmDialog
        open={confirmArchive}
        onOpenChange={(v) => {
          if (!v) {
            setConfirmArchive(false)
            setArchiveError(null)
          }
        }}
        title="确认归档"
        description="确定归档此空间？"
        confirmText="归档"
        destructive
        loading={archiveLoading}
        error={archiveError}
        onConfirm={() => {
          setArchiveError(null)
          archive({ variables: { id: space.id } })
        }}
      />
    </div>
  )
}
