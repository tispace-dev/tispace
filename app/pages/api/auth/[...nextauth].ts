import NextAuth from 'next-auth'
import axios from 'axios'
import GoogleProvider from 'next-auth/providers/google'

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
      } catch (_) {
        return false
      }

      return true
    },

    async jwt({ token, account }) {
      if (account) {
        token.id_token = account.id_token
      }

      return token
    },

    async session({ session, token }) {
      session.id_token = token.id_token

      return session
    },
  },
  pages: {
    signIn: '/login',
  },
  secret: process.env.SECRET,
})
