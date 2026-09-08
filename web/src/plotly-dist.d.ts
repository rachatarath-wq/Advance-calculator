// `plotly.js-dist-min` ships only a UMD bundle without types; declare it here so
// TypeScript resolves the import. Plotly calls stay loosely typed on purpose.
declare module 'plotly.js-dist-min' {
  const Plotly: any
  export default Plotly
}
