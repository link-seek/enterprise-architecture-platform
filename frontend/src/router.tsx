import { createBrowserRouter, Navigate, Outlet, useParams } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth'
import { TEST_SPACE_ID } from '@/api/spaces'

function AdminRoute() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  const user = useAuthStore((s) => s.user)
  // Anonymous users must log in before reaching admin-only pages.
  if (!isAuthenticated) return <Navigate to="/login" replace />
  // While authenticated but user data hasn't loaded yet (e.g. after refresh),
  // block access instead of letting a non-admin sneak through.
  if (!user) return null
  if (user.role !== 'admin') return <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/value-streams`} replace />
  return <Outlet />
}

export const router = createBrowserRouter([
  {
    path: '/',
    lazy: async () => ({ Component: (await import('@/views/home')).default }),
  },
  {
    path: '/login',
    lazy: async () => ({ Component: (await import('@/views/login')).default }),
  },
  // ── Spaces (public read, login to edit) ──────────────────────────────
  {
    path: '/spaces',
    lazy: async () => ({ Component: (await import('@/views/spaces/list')).default }),
  },
  {
    path: '/spaces/:spaceId',
    lazy: async () => ({ Component: (await import('@/views/spaces/detail')).default }),
  },
  // Space-scoped business architecture pages (public read, login to edit)
  {
    path: '/spaces/:spaceId/architectures',
    lazy: async () => ({ Component: (await import('@/views/architectures/layout')).default }),
    children: [
      { index: true, element: <Navigate to="overview" replace /> },
      {
        path: 'overview',
        lazy: async () => ({ Component: (await import('@/views/architectures/overview')).default }),
      },
      {
        path: 'value-streams',
        lazy: async () => ({ Component: (await import('@/views/architectures/value-streams')).default }),
      },
      {
        path: 'value-streams/:id',
        lazy: async () => ({ Component: (await import('@/views/architectures/value-stream-detail')).default }),
      },
      {
        path: 'capabilities',
        lazy: async () => ({ Component: (await import('@/views/architectures/capabilities')).default }),
      },
      {
        path: 'processes',
        lazy: async () => ({ Component: (await import('@/views/architectures/processes')).default }),
      },
      {
        path: 'applications',
        lazy: async () => ({ Component: (await import('@/views/architectures/applications')).default }),
      },
      {
        path: 'application-processes',
        lazy: async () => ({ Component: (await import('@/views/architectures/application-processes')).default }),
      },
      {
        path: 'organizational-units',
        lazy: async () => ({ Component: (await import('@/views/architectures/organizational-units')).default }),
      },
      {
        path: 'business-roles',
        lazy: async () => ({ Component: (await import('@/views/architectures/business-roles')).default }),
      },
      {
        path: 'functional-modules',
        lazy: async () => ({ Component: (await import('@/views/architectures/functional-modules')).default }),
      },
      {
        path: 'application-interfaces',
        lazy: async () => ({ Component: (await import('@/views/architectures/application-interfaces')).default }),
      },
      {
        path: 'realizations',
        lazy: async () => ({ Component: (await import('@/views/architectures/realizations')).default }),
      },
      {
        element: <AdminRoute />,
        children: [
          {
            path: 'users',
            lazy: async () => ({ Component: (await import('@/views/architectures/users')).default }),
          },
        ],
      },
    ],
  },
  // Legacy /architectures/* routes redirect into the test space for
  // backwards compatibility with existing bookmarks and E2E tests.
  {
    path: '/architectures',
    element: <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/value-streams`} replace />,
  },
  {
    path: '/architectures/value-streams',
    element: <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/value-streams`} replace />,
  },
  {
    path: '/architectures/value-streams/:id',
    element: <LegacyValueStreamRedirect />,
  },
  {
    path: '/architectures/capabilities',
    element: <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/capabilities`} replace />,
  },
  {
    path: '/architectures/processes',
    element: <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/processes`} replace />,
  },
  {
    path: '/architectures/users',
    element: <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/users`} replace />,
  },
  {
    path: '*',
    lazy: async () => ({ Component: (await import('@/views/not-found')).default }),
  },
])

function LegacyValueStreamRedirect() {
  const { id } = useParams<{ id: string }>()
  return <Navigate to={`/spaces/${TEST_SPACE_ID}/architectures/value-streams/${id}`} replace />
}
