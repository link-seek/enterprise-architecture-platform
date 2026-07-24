import { ApolloProvider } from '@apollo/client/react'
import { RouterProvider } from 'react-router-dom'
import { useEffect } from 'react'
import { apolloClient } from '@/lib/apollo'
import { router } from '@/router'
import { useAuthStore } from '@/stores/auth'
import { fetchMe } from '@/api/auth'

function useAuthInit() {
  const token = useAuthStore((s) => s.token)
  const user = useAuthStore((s) => s.user)
  const logout = useAuthStore((s) => s.logout)

  useEffect(() => {
    if (token && !user) {
      fetchMe().then((me) => {
        if (!me) logout()
      })
    }
  }, [token, user, logout])
}

export default function App() {
  useAuthInit()

  return (
    <ApolloProvider client={apolloClient}>
      <RouterProvider router={router} />
    </ApolloProvider>
  )
}
