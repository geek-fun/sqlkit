// Bridge transformer for @vue/vue3-jest under jest 30.
//
// vue3-jest emits a hybrid module: the <script> part is compiled to CommonJS
// (`exports.default = ...`), while compileTemplate is hardcoded to
// `mode: 'module'`, so the template part keeps ESM syntax — a helpers
// `import { x as y } from "vue"` line and `export function render(...)`.
// jest 29 executed such a file as CommonJS and tolerated the stray ESM
// fragments; jest 30 detects the ESM syntax and runs the whole file as an ES
// module instead, where the `exports` binding does not exist
// ("exports is not defined").
//
// Rewrite both fragments into CommonJS so the entire module is valid again:
// the helpers import becomes a `require("vue")` destructure and the render
// export becomes a plain function attached to the default export.
const vue3Jest = require('@vue/vue3-jest')

module.exports = {
  process(sourceText, sourcePath, options) {
    const result = vue3Jest.process(sourceText, sourcePath, options)
    const code = typeof result === 'string'
      ? result
      : result.code

    let cjs = code.replace(
      /import \{([^}]+)\} from ["']vue["'];?/g,
      (_match, names) => {
        const bindings = String(names)
          .split(',')
          .map(entry => entry.trim())
          .filter(Boolean)
          .map(entry => entry.replace(/\s+as\s+/, ': '))
          .join(', ')
        // The template part is glued to the inline `//# sourceMappingURL=`
        // comment of the script part — start a fresh line or the whole
        // rewrite would still sit inside that line comment.
        return `\nconst { ${bindings} } = require("vue");`
      },
    )
    cjs = cjs.replace(/export function (ssrRender|render)\(/, 'function $1(')

    if (cjs !== code) {
      cjs += ';exports.default = { ...exports.default, render };'
    }
    return { code: cjs, map: result.map }
  },
}
