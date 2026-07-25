import { useState } from 'react'
import { useQuery } from '@apollo/client/react'
import { Link, useNavigate } from 'react-router-dom'
import { Plus, LogIn, LayoutGrid } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { useAuthStore } from '@/stores/auth'
import { GET_SPACES } from '@/api/spaces'
import type { Space } from '@/api/spaces'
import { SpaceCreateDialog } from './crud'

export default function SpacesList() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const navigate = useNavigate()
  const [createOpen, setCreateOpen] = useState(false)

  const { data, loading, error } = useQuery<{ organizations: { nodes: Space[] } }>(GET_SPACES)

  const spaces = data?.organizations?.nodes ?? []

  return (
    <div className="min-h-screen bg-secondary flex flex-col">
      <header className="border-b bg-background">
        <div className="container mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <div className="flex items-center gap-2">
            <LayoutGrid className="h-5 w-5" />
            <span className="text-lg font-semibold">空间 · Spaces</span>
          </div>
          <div className="flex items-center gap-2">
            {isAuthenticated ? (
              <Button onClick={() => setCreateOpen(true)}>
                <Plus className="h-4 w-4 mr-2" />
                创建空间
              </Button>
            ) : (
              <Button variant="outline" onClick={() => navigate('/login')}>
                <LogIn className="h-4 w-4 mr-2" />
                登录以编辑
              </Button>
            )}
          </div>
        </div>
      </header>

      <main className="flex-1 container mx-auto max-w-6xl px-4 py-10">
        <h1 className="text-2xl font-semibold">所有空间</h1>
        <p className="mt-1 text-muted-foreground">
          每个空间对应一个企业的完整架构。未登录可查看案例展示，登录后可在所属空间内编辑。
        </p>

        {loading && <div className="mt-8 text-center text-muted-foreground">加载中...</div>}
        {error && <div className="mt-8 text-center text-destructive">加载失败: {error.message}</div>}

        <div className="mt-8 grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {spaces.map((space) => (
            <Link key={space.id} to={`/spaces/${space.id}`}>
              <Card className="h-full hover:shadow-md transition-shadow">
                <CardHeader>
                  <CardTitle className="flex items-center justify-between">
                    {space.name}
                    {space.deletedAt && <Badge variant="outline">已归档</Badge>}
                  </CardTitle>
                  <CardDescription>
                    {space.description || '暂无描述'}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <p className="text-xs text-muted-foreground">
                    创建于 {new Date(space.createdAt).toLocaleDateString()}
                  </p>
                </CardContent>
              </Card>
            </Link>
          ))}
          {!loading && spaces.length === 0 && (
            <div className="col-span-full text-center py-16 text-muted-foreground">
              暂无空间
            </div>
          )}
        </div>
      </main>

      <SpaceCreateDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  )
}