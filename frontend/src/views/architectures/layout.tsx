import { Link, useLocation, useParams } from 'react-router-dom'
import { Outlet } from 'react-router-dom'
import { useQuery } from '@apollo/client/react'
import { useState } from 'react'
import { useAuthStore } from '@/stores/auth'
import { logout } from '@/api/auth'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import {
  LayoutDashboard,
  Boxes,
  Workflow,
  LogOut,
  Users,
  ArrowLeft,
  Menu,
} from 'lucide-react'
import { GET_SPACE } from '@/api/spaces'
import type { Space } from '@/api/spaces'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'

function SidebarContent({ onNavigate, spaceName, spaceId }: { onNavigate?: () => void; spaceName: string; spaceId?: string }) {
  const location = useLocation()
  const user = useAuthStore((s) => s.user)
  const { canEdit } = useSpaceMembership(spaceId)

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
    <div className="flex h-full flex-col">
      <div className="p-4">
        <Link to="/spaces" onClick={onNavigate} className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground mb-1">
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
      <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
        {menuItems.map((item) => {
          const Icon = item.icon
          const active = location.pathname.startsWith(item.path)
          return (
            <Link
              key={item.path}
              to={item.path}
              onClick={onNavigate}
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
                  onClick={onNavigate}
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
          onClick={async () => { try { await logout() } finally { onNavigate?.() } }}
        >
          <LogOut className="h-4 w-4" />
          退出登录
        </Button>
      </div>
    </div>
  )
}

export default function ArchLayout() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const isMobile = useIsMobile()
  const [drawerOpen, setDrawerOpen] = useState(false)

  const { data: spaceData } = useQuery<{ spaceById: Space | null }>(GET_SPACE, {
    variables: { id: spaceId },
    skip: !spaceId,
  })
  const spaceName = spaceData?.spaceById?.name ?? '空间'

  return (
    <div className="flex h-screen">
      {/* Desktop sidebar */}
      {!isMobile && (
        <aside className="w-60 border-r bg-card flex flex-col">
          <SidebarContent spaceName={spaceName} spaceId={spaceId} />
        </aside>
      )}

      {/* Mobile drawer */}
      <Dialog open={drawerOpen} onOpenChange={setDrawerOpen}>
        <DialogContent className="left-0 top-0 h-full max-w-[280px] max-h-none translate-x-0 translate-y-0 rounded-none sm:rounded-none p-0 data-[state=open]:slide-in-from-left-full data-[state=closed]:slide-out-to-left-full data-[state=open]:slide-in-from-top-0 data-[state=closed]:slide-out-to-top-0 data-[state=open]:zoom-in-100 data-[state=closed]:zoom-out-100">
          <DialogTitle className="sr-only">导航菜单</DialogTitle>
          <div className="h-full bg-card">
            <SidebarContent onNavigate={() => setDrawerOpen(false)} spaceName={spaceName} spaceId={spaceId} />
          </div>
        </DialogContent>
      </Dialog>

      {/* Main content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Mobile header */}
        <header className="md:hidden sticky top-0 z-10 flex h-14 items-center gap-3 border-b bg-card px-4">
          <Button variant="ghost" size="icon" onClick={() => setDrawerOpen(true)} aria-label="打开菜单">
            <Menu className="h-5 w-5" />
          </Button>
          <h1 className="text-base font-semibold truncate">{spaceName}</h1>
        </header>
        <div className="flex-1 overflow-auto">
          <Outlet />
        </div>
        <footer className="border-t px-4 md:px-6 py-3 text-center text-xs text-muted-foreground">
          © 2025 企业架构平台
        </footer>
      </main>
    </div>
  )
}
