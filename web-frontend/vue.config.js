module.exports = {
  devServer: {
    proxy: process.env.PROXY || 'http://localhost:8081'
  }
}
