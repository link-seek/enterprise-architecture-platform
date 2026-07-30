import { gql } from '@apollo/client'

// ============================================================================
// Fragments
// ============================================================================

export const SPACE_FIELDS = gql`
  fragment SpaceFields on Organizations {
    id
    name
    description
    visibility
    createdAt
    updatedAt
    deletedAt
  }
`

export const SPACE_MEMBER_FIELDS = gql`
  fragment SpaceMemberFields on SpaceMemberWithUser {
    userId
    name
    role
  }
`

// ============================================================================
// Queries
// ============================================================================
//
// These use the visibility-aware custom queries (`spaces`/`spaceById`/`*BySpace`)
// rather than the seaography auto-generated query, which is admin-only. Public
// spaces are readable by anyone (including anonymous); private spaces require
// membership.

export const GET_SPACES = gql`
  ${SPACE_FIELDS}
  query GetSpaces {
    spaces {
      ...SpaceFields
    }
  }
`

export const GET_SPACE = gql`
  ${SPACE_FIELDS}
  query GetSpace($id: String!) {
    spaceById(id: $id) {
      ...SpaceFields
    }
  }
`

export const GET_SPACE_MEMBERS = gql`
  ${SPACE_MEMBER_FIELDS}
  query GetSpaceMembers($spaceId: String!) {
    spaceMembersBySpace(spaceId: $spaceId) {
      ...SpaceMemberFields
    }
  }
`

export const GET_SPACE_STATS = gql`
  query GetSpaceStats($spaceId: String!) {
    valueStreamsBySpace(spaceId: $spaceId) {
      id
    }
    businessCapabilitiesBySpace(spaceId: $spaceId) {
      id
    }
    businessProcessesBySpace(spaceId: $spaceId) {
      id
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

export const CREATE_SPACE = gql`
  ${SPACE_FIELDS}
  mutation SpaceCreate($name: String!, $description: String, $visibility: String!) {
    spaceCreate(name: $name, description: $description, visibility: $visibility) {
      ...SpaceFields
    }
  }
`

export const UPDATE_SPACE = gql`
  ${SPACE_FIELDS}
  mutation SpaceUpdate($id: String!, $name: String, $description: String) {
    spaceUpdate(id: $id, name: $name, description: $description) {
      ...SpaceFields
    }
  }
`

export const SET_SPACE_VISIBILITY = gql`
  ${SPACE_FIELDS}
  mutation SpaceSetVisibility($id: String!, $visibility: String!) {
    spaceSetVisibility(id: $id, visibility: $visibility) {
      ...SpaceFields
    }
  }
`

export const ARCHIVE_SPACE = gql`
  mutation SpaceArchive($id: String!) {
    spaceArchive(id: $id)
  }
`

export const ADD_SPACE_MEMBER = gql`
  mutation SpaceAddMember($spaceId: String!, $userId: String!, $role: String!) {
    spaceAddMember(spaceId: $spaceId, userId: $userId, role: $role) {
      spaceId
      userId
      role
    }
  }
`

export const REMOVE_SPACE_MEMBER = gql`
  mutation SpaceRemoveMember($spaceId: String!, $userId: String!) {
    spaceRemoveMember(spaceId: $spaceId, userId: $userId)
  }
`

// ============================================================================
// Types
// ============================================================================

export type SpaceVisibility = 'public' | 'private'

export interface Space {
  id: string
  name: string
  description: string | null
  visibility: SpaceVisibility
  createdAt: string
  updatedAt: string
  deletedAt: string | null
}

export interface SpaceMember {
  userId: string
  name: string
  role: 'owner' | 'editor'
}

export interface SpaceStats {
  valueStreamsBySpace: { id: string }[]
  businessCapabilitiesBySpace: { id: string }[]
  businessProcessesBySpace: { id: string }[]
}

// Fixed UUID of the seeded "测试空间" (test space) that owns pre-existing
// business data. Mirrors `migration::m20250101_000029...::TEST_SPACE_ID`.
export const TEST_SPACE_ID = '00000000-0000-0000-0000-000000000010'