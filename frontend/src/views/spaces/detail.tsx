import { useState } from 'react'
import { useParams, Link, useNavigate } from 'react-router-dom'
import { useQuery, useMutation } from '@apollo/client/react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Pencil, Archive, LogIn, ArrowLeft, Users } from 'lucide-react'
import { GET_SPACE, ARCHIVE_SPACE, GET_SPACES, GET_SPACE_STATS } from '@/api/spaces'
import type { Space, SpaceStats } from '@/api/spaces'
import { useAuthStore } from '@/stores/auth'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { SpaceEditDialog } from './crud'
import { SpaceMembersDialog } from './members'

export default function SpaceDetail() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const navigate = useNavigate()
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const { canEdit, role } = useSpaceMembership(spaceId)
  const [editOpen, setEditOpen] = useState(false)
  const [membersOpen, setMembersOpen] = useState(false)

  const { data, loading, error } = useQuery<{ spaceById: Space | null }>(GET_SPACE, {
    variables: { id: spaceId },
    skip: !spaceId,
  })
  const { data: stats } = useQuery<SpaceStats>(GET_SPACE_STATS, {
    variables: { spaceId },
    skip: !spaceId,
  })

  const [archive] = useMutation(ARCHIVE_SPACE, {
    refetchQueries: [{ query: GET_SPACES }],
    onCompleted: () => navigate('/spaces'),
  })

  const space = data?.spaceById

  if (loading) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">加载中...</div>
  if (error) return <div className="min-h-screen flex items-center justify-center text-destructive">加载失败: {error.message}</div>
  if (!space) return <div className="min-h-screen flex items-center justify-center text-muted-foreground">空间不存在</div>

  const statsItems = [
    { label: '价值流', value: stats?.valueStreamsBySpace?.length ?? 0, to: 'value-streams' },
    { label: '业务能力', value: stats?.businessCapabilitiesBySpace?.length ?? 0, to: 'capabilities' },
    { label: '业务流程', value: stats?.businessProcessesBySpace?.length ?? 0, to: 'processes' },
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
          <div className="flex items-center gap-2">
            {canEdit && (
              <>
                <Button variant="outline" size="sm" onClick={() => setEditOpen(true)}>
                  <Pencil className="h-4 w-4 mr-2" />
                  编辑
                </Button>
                {role === 'owner' && (
                  <Button variant="outline" size="sm" onClick={() => setMembersOpen(true)}>
                    <Users className="h-4 w-4 mr-2" />
                    成员
                  </Button>
                )}
                {role === 'owner' && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      if (confirm('确定归档此空间？')) archive({ variables: { id: space.id } })
                    }}
                  >
                    <Archive className="h-4 w-4 mr-2" />
                    归档
                  </Button>
                )}
              </>
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