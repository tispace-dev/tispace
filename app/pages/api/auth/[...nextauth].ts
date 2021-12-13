import NextAuth, { User } from 'next-auth'
import Providers from 'next-auth/providers'
import axios from 'axios'

export default NextAuth({
  providers: [
    Providers.Credentials({
      name: 'Credentials',
      async authorize(credentials: { username: string; password: string }) {
        const user: User = await axios.post(
          `${process.env.SERVER_URL}/authorize`,
          {
            username: credentials.username,
            password: credentials.password,
          }
        )

        if (user) {
          return user
        } else {
          return null
        }
      },
    }),
  ],
  callbacks: {
    async jwt(token, user) {
      if (user) {
        token.accessToken = user.token
      }

      return token
    },

    async session(session, token) {
      session.accessToken = token.accessToken
      return session
    },
  },
})
