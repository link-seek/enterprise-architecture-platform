import { Link, useLocation, useParams } from 'react-router-dom'
import { Outlet } from 'react-router-dom'
import { useQuery } from '@apollo/client/react'
import { useAuthStore } from '@/stores/auth'
import { logout } from '@/api/auth'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import {
  LayoutDashboard,
  Boxes,
  Workflow,
  LogOut,
  Users,
  ArrowLeft,
} from 'lucide-react'
import { GET_SPACE } from '@/api/spaces'
import type { Space } from '@/api/spaces'
import { useSpaceMembership } from '@/hooks/use-space-membership'

export default function ArchLayout() {
  const location = useLocation()
  const user = useAuthStore((s) => s.user)
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)

  const { data: spaceData } = useQuery<{ organizations: { nodes: Space[] } }>(GET_SPACE, {
    variables: { id: spaceId },
    skip: !spaceId,
  })
  const spaceName = spaceData?.organizations?.nodes?.[0]?.name ?? '空间'

  const base = spaceId ? `/spaces/${spaceId}/architectures` : '/architectures'
  const menuItems = [
    { path: `${base}/value-streams`, label: '价值流', icon: LayoutDashboard },
    { path: `${base}/capabilities`, label: '业务能力', icon: Boxes },
    { path: `${base}/processes`, label: '业务流程', icon: Workflow },
  ]

  const adminMenuItems = [
    { path: `${base}/users`, label: '用户管理', icon: Users },
  ]

  return (
    <div className="flex h-screen">
      {/* Sidebar */}
      <aside className="w-60 border-r bg-card flex flex-col">
        <div className="p-4">
          <Link to="/spaces" className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground mb-1">
            <ArrowLeft className="h-3.5 w-3.5" />
            所有空间
          </Link>
          <h1 className="text-lg font-semibold truncate">{spaceName}</h1>
          <p className="text-sm text-muted-foreground">Enterprise Architecture</p>
          {!canEdit && (
            <p className="mt-1 text-xs text-amber-600">只读模式（非成员）</p>
          )}
        </div>
        <Separator />
        <nav className="flex-1 p-2 space-y-1">
          {menuItems.map((item) => {
            const Icon = item.icon
            const active = location.pathname.startsWith(item.path)
            return (
              <Link
                key={item.path}
                to={item.path}
                className={`flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                  active
                    ? 'bg-primary text-primary-foreground'
                    : 'hover:bg-accent hover:text-accent-foreground'
                }`}
              >
                <Icon className="h-4 w-4" />
                {item.label}
              </Link>
            )
          })}
          {user?.role === 'admin' && (
            <>
              <Separator className="my-2" />
              {adminMenuItems.map((item) => {
                const Icon = item.icon
                const active = location.pathname.startsWith(item.path)
                return (
                  <Link
                    key={item.path}
                    to={item.path}
                    className={`flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                      active
                        ? 'bg-primary text-primary-foreground'
                        : 'hover:bg-accent hover:text-accent-foreground'
                    }`}
                  >
                    <Icon className="h-4 w-4" />
                    {item.label}
                  </Link>
                )
              })}
            </>
          )}
        </nav>
        <Separator />
        <div className="p-3">
          <div className="flex items-center gap-2 mb-2">
            <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center text-sm font-medium">
              {user?.name?.[0] || 'U'}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium truncate">{user?.name || 'User'}</p>
              <p className="text-xs text-muted-foreground truncate">{user?.email}</p>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-start gap-2 text-muted-foreground"
            onClick={() => logout()}
          >
            <LogOut className="h-4 w-4" />
            退出登录
          </Button>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        <div className="flex-1 overflow-auto">
          <Outlet />
        </div>
        <footer className="border-t px-6 py-3 text-center text-xs text-muted-foreground">
          © 2025 企业架构平台
        </footer>
      </main>
    </div>
  )
}
