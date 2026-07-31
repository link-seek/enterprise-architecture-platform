import { useState } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { useQuery, useMutation } from '@apollo/client/react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Pencil, Archive, LogIn, ArrowLeft, Users, MoreVertical } from 'lucide-react'
import { GET_SPACE, ARCHIVE_SPACE, GET_SPACES, GET_SPACE_STATS } from '@/api/spaces'
import type { Space, SpaceStats } from '@/api/spaces'
import { useAuthStore } from '@/stores/auth'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'
import { SpaceEditDialog } from './crud'
import { SpaceMembersDialog } from './members'

export default function SpaceDetail() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const navigate = useNavigate()
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const { canEdit, role } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [editOpen, setEditOpen] = useState(false)
  const [membersOpen, setMembersOpen] = useState(false)

  const { data, loading, error } = useQuery<{ organizations: { nodes: Space[] } }>(GET_SPACE, {
    variables: { id: spaceId },
    skip: !spaceId,
  })
  const { data: stats } = useQuery<SpaceStats>(GET_SPACE_STATS, {
    variables: { spaceId },
    skip: !spaceId,
  })

  const [archiveError, setArchiveError] = useState<string | null>(null)
  const [archive] = useMutation(ARCHIVE_SPACE, {
    refetchQueries: [{ query: GET_SPACES }],
    onCompleted: () => navigate('/spaces'),
    onError: (err) => setArchiveError(err instanceof Error ? err.message : '归档失败'),
  })

  const space = data?.organizations?.nodes?.[0]

  if (loading) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">加载中...</div>
  if (error) return <div className="min-h-screen flex items-center justify-center text-destructive">加载失败: {error.message}</div>
  if (!space) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">空间不存在</div>

  const handleEdit = () => setEditOpen(true)
  const handleMembers = () => setMembersOpen(true)
  const handleArchive = () => {
    if (confirm('确定归档此空间？')) {
      setArchiveError(null)
      archive({ variables: { id: space.id } })
    }
  }

  const statsItems = [
    { label: '价值流', value: stats?.valueStreams?.paginationInfo?.total ?? 0, to: 'value-streams' },
    { label: '业务能力', value: stats?.businessCapabilities?.paginationInfo?.total ?? 0, to: 'capabilities' },
    { label: '业务流程', value: stats?.businessProcesses?.paginationInfo?.total ?? 0, to: 'processes' },
  ]

  const spaceActions = [
    { icon: Pencil, label: '编辑', onClick: handleEdit, visible: true },
    { icon: Users, label: '成员', onClick: handleMembers, visible: role === 'owner' },
    { icon: Archive, label: '归档', onClick: handleArchive, visible: role === 'owner' },
  ]

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
                    {spaceActions.filter((a) => a.visible).map((action) => {
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
                  {spaceActions.filter((a) => a.visible).map((action) => {
                    const Icon = action.icon
                    return (
                      <Button key={action.label} variant="outline" size="sm" onClick={action.onClick}>
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
          <div className="mb-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">{archiveError}</div>
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
    </div>
  )
}