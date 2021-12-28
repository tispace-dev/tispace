import { createServer } from 'http'
import next from 'next'
import absoluteUrl from 'next-absolute-url'
import { parse } from 'url'

const dev = process.env.NODE_ENV !== 'production'
const app = next({ dev })
const handle = app.getRequestHandler()
const port = process.env.PORT || 3000

app.prepare().then(() => {
  createServer((req, res) => {
    const { url } = req
    const { protocol, host } = absoluteUrl(req)

    if (protocol === 'https:') {
      res.writeHead(301, {
        Location: `http://${host}${url}`,
      })
      res.end()
      return
    }

    const parsedUrl = parse(url!, true)
    return handle(req, res, parsedUrl)
  })
    .listen(port)
    .on('listening', () => {
      console.log(
        `> Server listening at http://localhost:${port} as ${
          dev ? 'development' : process.env.NODE_ENV
        }`
      )
    })
})
