declare module '*.module.less' {
  const classes: { readonly [key: string]: string }
  export default classes
}

namespace NodeJS {
  interface ProcessEnv extends NodeJS.ProcessEnv {
    GOOGLE_CLIENT_ID: string
    GOOGLE_CLIENT_SECRET: string
  }
}
