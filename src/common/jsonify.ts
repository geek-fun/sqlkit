import JSON5 from 'json5'
import { JSONParse, JSONStringify } from 'json-with-bigint'

function bigIntStringify(text: string) {
  return text.replace(/([-+]?\d+)\b/g, (match) => {
    return Number.isSafeInteger(Number(match)) ? match : `"${match}n"`
  })
}

function bigIntParse(text: string) {
  return text.replace(/(?<=([:,[]\s*))["']([-+]?\d+)n["']/g, '$2')
}

export const jsonify = {
  stringify: JSONStringify,
  parse: JSONParse,
  parse5: (text: string) => JSON5.parse(bigIntStringify(text)),
  string5: (value: any, replacer?: any, space?: string | number): string =>
    bigIntParse(JSON5.stringify(value, replacer, space)),
} as unknown as {
  stringify: typeof JSON.stringify
  parse: typeof JSON.parse
  parse5: typeof JSON5.parse
  string5: typeof JSON5.stringify
}
