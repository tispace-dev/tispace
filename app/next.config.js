const debug = process.env.NODE_ENV !== 'production'
const withAntdLess = require('next-plugin-antd-less')

module.exports = {
  ...withAntdLess({
    webpack(config) {
      return config
    },
  }),
  assetPrefix: !debug ? '/tispace/' : '',
  basePath: !debug ? '/tispace/' : '',
}
