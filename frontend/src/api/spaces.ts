import { gql } from '@apollo/client'

// ============================================================================
// Fragments
// ============================================================================

export const SPACE_FIELDS = gql`
  fragment SpaceFields on Organizations {
    id
    name
    description
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

export const GET_SPACES = gql`
  ${SPACE_FIELDS}
  query GetSpaces {
    organizations(filters: { deletedAt: { is_null: true } }) {
      nodes {
        ...SpaceFields
      }
      paginationInfo {
        total
      }
    }
  }
`

export const GET_SPACE = gql`
  ${SPACE_FIELDS}
  query GetSpace($id: String!) {
    organizations(filters: { id: { eq: $id } }) {
      nodes {
        ...SpaceFields
      }
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
    valueStreams(filters: { spaceId: { eq: $spaceId } }) {
      paginationInfo { total }
    }
    businessCapabilities(filters: { spaceId: { eq: $spaceId } }) {
      paginationInfo { total }
    }
    businessProcesses(filters: { spaceId: { eq: $spaceId } }) {
      paginationInfo { total }
    }
  }
`

// ============================================================================
// Mutations
// ============================================================================

export const CREATE_SPACE = gql`
  ${SPACE_FIELDS}
  mutation SpaceCreate($name: String!, $description: String) {
    spaceCreate(name: $name, description: $description) {
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

export interface Space {
  id: string
  name: string
  description: string | null
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
  valueStreams: { paginationInfo: { total: number } }
  businessCapabilities: { paginationInfo: { total: number } }
  businessProcesses: { paginationInfo: { total: number } }
}

// Fixed UUID of the seeded "测试空间" (test space) that owns pre-existing
// business data. Mirrors `migration::m20250101_000029...::TEST_SPACE_ID`.
export const TEST_SPACE_ID = '00000000-0000-0000-0000-000000000010'