import NextAuth from 'next-auth'
import axios from 'axios'
import GoogleProvider from 'next-auth/providers/google'

const RefreshIdTokenError = 'RefreshIdTokenError'

type Token = {
  refreshToken: string
}

/**
 * Takes a token, and returns a new token with updated
 * `idToken` and `idTokenExpires`. If an error occurs,
 * returns the old token and an error property.
 * @param token
 */
async function refreshAccessToken(token: Token) {
  try {
    const url =
      'https://oauth2.googleapis.com/token?' +
      new URLSearchParams({
        client_id: process.env.GOOGLE_CLIENT_ID,
        client_secret: process.env.GOOGLE_CLIENT_SECRET,
        grant_type: 'refresh_token',
        refresh_token: token.refreshToken,
      })

    const refreshedTokensRes = await axios.post(url)

    if (refreshedTokensRes.status !== 200) {
      return {
        ...token,
        error: RefreshIdTokenError,
      }
    }

    const { data: refreshedTokens } = refreshedTokensRes
    return {
      idToken: refreshedTokens.id_token,
      idTokenExpires: Date.now() + refreshedTokens.expires_in * 1000,
      refreshToken: refreshedTokens.refresh_token ?? token.refreshToken, // Fall back to old refresh token
    }
  } catch (error) {
    return {
      ...token,
      error: RefreshIdTokenError,
    }
  }
}

export default NextAuth({
  providers: [
    GoogleProvider({
      clientId: process.env.GOOGLE_CLIENT_ID,
      clientSecret: process.env.GOOGLE_CLIENT_SECRET,
      authorization: {
        params: {
          prompt: 'consent',
          access_type: 'offline',
          response_type: 'code',
        },
      },
    }),
  ],
  callbacks: {
    async signIn({ account }) {
      try {
        const authorized = await axios.get(
          `${process.env.NEXT_PUBLIC_SERVER_URL}/authorized`,
          {
            headers: {
              Authorization: `Bearer ${account.id_token}`,
            },
          }
        )
        if (authorized.status !== 200) {
          return false
        }
      } catch (e) {
        console.log(e)
        return false
      }

      return true
    },

    jwt: async function ({ token, account }) {
      if (account) {
        token.idToken = account.id_token
        token.idTokenExpires =
          Date.now() + (account.expires_in as number) * 1000
        token.refreshToken = account.refresh_token
        return token
      }

      // Return previous token if the access token has not expired yet
      if (Date.now() < (token.idTokenExpires as number)) {
        return token
      }

      // Access token has expired, try to update it
      return refreshAccessToken(token as Token)
    },

    async session({ session, token }) {
      session.idToken = token.idToken
      session.error = token.error

      return session
    },
  },
  pages: {
    signIn: '/login',
  },
  secret: process.env.SECRET,
})
