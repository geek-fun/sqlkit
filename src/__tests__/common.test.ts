import { pureObject } from '../common'

describe('pureObject', () => {
  it('should convert a simple object to a plain object', () => {
    const input = { a: 1, b: 'test' }
    const result = pureObject(input)
    expect(result).toEqual(input)
    expect(result).not.toBe(input) // Should be a new object
  })

  it('should convert nested objects', () => {
    const input = { a: { b: { c: 1 } } }
    const result = pureObject(input)
    expect(result).toEqual(input)
    expect(result.a).not.toBe(input.a)
  })

  it('should convert arrays', () => {
    const input = [1, 2, 3]
    const result = pureObject(input)
    expect(result).toEqual(input)
    expect(result).not.toBe(input)
  })

  it('should handle mixed objects and arrays', () => {
    const input = { items: [{ id: 1 }, { id: 2 }] }
    const result = pureObject(input)
    expect(result).toEqual(input)
    expect(result.items).not.toBe(input.items)
  })

  it('should handle primitives', () => {
    expect(pureObject('string')).toBe('string')
    expect(pureObject(123)).toBe(123)
    expect(pureObject(true)).toBe(true)
    expect(pureObject(null)).toBe(null)
  })

  it('should remove functions from objects', () => {
    const input = {
      value: 1,
      method: () => undefined,
    }
    const result = pureObject(input)
    expect(result).toEqual({ value: 1 })
    expect(result).not.toHaveProperty('method')
  })
})
