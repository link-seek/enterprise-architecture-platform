import { create } from 'zustand'

interface User {
  id: string
  email: string
  name: string
  role: string
  status: string
}

interface AuthState {
  token: string | null
  refreshToken: string | null
  user: User | null
  isAuthenticated: boolean
  currentSpaceId: string | null
  login: (token: string, refreshToken: string, user: User) => void
  logout: () => void
  setUser: (user: User) => void
  setCurrentSpaceId: (spaceId: string | null) => void
}

export const useAuthStore = create<AuthState>((set) => ({
  token: localStorage.getItem('access_token'),
  refreshToken: localStorage.getItem('refresh_token'),
  user: null,
  isAuthenticated: !!localStorage.getItem('access_token'),
  currentSpaceId: localStorage.getItem('current_space_id'),
  login: (token, refreshToken, user) => {
    localStorage.setItem('access_token', token)
    localStorage.setItem('refresh_token', refreshToken)
    set({ token, refreshToken, user, isAuthenticated: true })
  },
  logout: () => {
    localStorage.removeItem('access_token')
    localStorage.removeItem('refresh_token')
    set({ token: null, refreshToken: null, user: null, isAuthenticated: false })
  },
  setUser: (user) => set({ user }),
  setCurrentSpaceId: (spaceId) => {
    if (spaceId) localStorage.setItem('current_space_id', spaceId)
    else localStorage.removeItem('current_space_id')
    set({ currentSpaceId: spaceId })
  },
}))
