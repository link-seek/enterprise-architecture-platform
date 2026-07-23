import { createBrowserRouter, Navigate, Outlet } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth'

function ProtectedRoute() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)
  if (!isAuthenticated) return <Navigate to="/login" replace />
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
  {
    element: <ProtectedRoute />,
    children: [
      {
        path: '/architectures',
        lazy: async () => ({ Component: (await import('@/views/architectures/layout')).default }),
        children: [
          { index: true, element: <Navigate to="/architectures/value-streams" replace /> },
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
            path: 'users',
            lazy: async () => ({ Component: (await import('@/views/architectures/users')).default }),
          },
        ],
      },
    ],
  },
  {
    path: '*',
    lazy: async () => ({ Component: (await import('@/views/not-found')).default }),
  },
])
