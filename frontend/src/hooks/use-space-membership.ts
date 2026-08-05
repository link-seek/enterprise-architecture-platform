import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useAuthStore } from '@/stores/auth'

const GET_MY_MEMBERSHIP = gql`
  query GetMyMembership($spaceId: String!) {
    myMembership(spaceId: $spaceId) {
      role
    }
  }
`

interface MembershipData {
  myMembership: { role: string } | null
}

// Returns the current user's role in the given space (`owner` | `editor` |
// null) plus a `canEdit` convenience flag. Anonymous users and non-members
// resolve to `null` / `false`. Uses the membership-enforced `myMembership`
// custom query (the caller's own role only) rather than the admin-gated
// auto-generated `spaceMembers` query, so non-admin editors/owners resolve
// their edit permissions correctly.
//
// `isEntityOwner(ownerId)` resolves entity-level ownership: true when the
// actor is the entity owner (with space edit rights) or a platform admin
// (admins bypass entity-owner checks on the backend).
export function useSpaceMembership(spaceId: string | undefined) {
  const user = useAuthStore((s) => s.user)
  const { data, loading } = useQuery<MembershipData>(GET_MY_MEMBERSHIP, {
    variables: { spaceId },
    skip: !spaceId || !user?.id,
  })

  const role = data?.myMembership?.role ?? null
  const canEdit = role === 'owner' || role === 'editor'
  const isAdmin = user?.role === 'admin'
  const isEntityOwner = (ownerId?: string | null) =>
    isAdmin || (canEdit && user?.id != null && ownerId === user.id)

  return { role, canEdit, loading, user, isEntityOwner }
}