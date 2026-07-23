import { useAuthStore } from '@/stores/auth'

const API_BASE = import.meta.env.VITE_API_URL || '/api'

interface LoginResponse {
  access_token: string
  refresh_token: string
  expires_in: number
  user: {
    id: string
    email: string
    name: string
    role: string
    status: string
  }
}

interface RegisterResponse {
  access_token: string
  refresh_token: string
  user: LoginResponse['user']
}

export async function login(email: string, password: string): Promise<LoginResponse> {
  const res = await fetch(`${API_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Login failed' }))
    throw new Error(err.message || 'Login failed')
  }
  const data = await res.json()
  useAuthStore.getState().login(data.access_token, data.refresh_token, data.user)
  return data
}

export async function register(email: string, name: string, password: string): Promise<RegisterResponse> {
  const res = await fetch(`${API_BASE}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, name, password }),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Registration failed' }))
    throw new Error(err.message || 'Registration failed')
  }
  const data = await res.json()
  useAuthStore.getState().login(data.access_token, data.refresh_token, data.user)
  return data
}

export async function logout(): Promise<void> {
  const token = useAuthStore.getState().token
  if (token) {
    await fetch(`${API_BASE}/auth/logout`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` },
    }).catch(() => {})
  }
  useAuthStore.getState().logout()
}

export async function fetchMe(): Promise<LoginResponse['user'] | null> {
  const token = useAuthStore.getState().token
  if (!token) return null
  const res = await fetch(`${API_BASE}/auth/me`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!res.ok) return null
  const data = await res.json()
  useAuthStore.getState().setUser(data)
  return data
}

export async function createUser(
  email: string,
  name: string,
  password: string,
  role?: string,
): Promise<LoginResponse['user']> {
  const token = useAuthStore.getState().token
  const res = await fetch(`${API_BASE}/auth/users`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify({ email, name, password, role }),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Create user failed' }))
    throw new Error(err.message || 'Create user failed')
  }
  return res.json()
}

export async function updateRole(userId: string, role: string): Promise<LoginResponse['user']> {
  const token = useAuthStore.getState().token
  const res = await fetch(`${API_BASE}/auth/role`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify({ user_id: userId, role }),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Update role failed' }))
    throw new Error(err.message || 'Update role failed')
  }
  return res.json()
}
