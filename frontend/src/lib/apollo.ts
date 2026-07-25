import { ApolloClient, InMemoryCache, createHttpLink, from } from '@apollo/client'
import { setContext } from '@apollo/client/link/context'

// In production, frontend is served from OSS (www.xieyucheng.top) but
// GraphQL API lives on the backend (api.xieyucheng.top). Use VITE_GRAPHQL_URL
// or derive from VITE_API_URL to avoid sending requests to OSS (405 error).
const graphqlUri = import.meta.env.VITE_GRAPHQL_URL
  ?? (import.meta.env.VITE_API_URL
    ? import.meta.env.VITE_API_URL.replace(/\/api\/?$/, '/graphql')
    : '/graphql')

const httpLink = createHttpLink({
  uri: graphqlUri,
})

const authLink = setContext((_, { headers }) => {
  const token = localStorage.getItem('access_token')
  return {
    headers: {
      ...headers,
      authorization: token ? `Bearer ${token}` : '',
    },
  }
})

export const apolloClient = new ApolloClient({
  link: from([authLink, httpLink]),
  cache: new InMemoryCache(),
  defaultOptions: {
    watchQuery: {
      errorPolicy: 'all',
    },
    query: {
      errorPolicy: 'all',
    },
  },
})
