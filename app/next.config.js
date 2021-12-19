const withAntdLess = require('next-plugin-antd-less')

module.exports = {
  ...withAntdLess({
    webpack(config) {
      return config
    },
  }),
}
