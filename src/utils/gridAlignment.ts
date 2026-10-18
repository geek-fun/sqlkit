export type ColumnAlignment = 'start' | 'center' | 'end'

const NUMERIC_TYPE_PATTERN
  = /^(?:INT|BIGINT|SMALLINT|TINYINT|SERIAL|BIGSERIAL|FLOAT|DOUBLE|REAL|DECIMAL|NUMERIC|MONEY)/
const BOOLEAN_TYPE_PATTERN = /^(?:BOOL|BIT)/

const ALIGNMENT_CLASSES: Record<ColumnAlignment, string> = {
  start: '',
  center: 'justify-center',
  end: 'justify-end',
}

export const getColumnAlignment = (type?: string): ColumnAlignment => {
  if (!type)
    return 'start'
  const normalized = type.toUpperCase()
  if (NUMERIC_TYPE_PATTERN.test(normalized))
    return 'end'
  if (BOOLEAN_TYPE_PATTERN.test(normalized))
    return 'center'
  return 'start'
}

export const getAlignmentClass = (type?: string): string =>
  ALIGNMENT_CLASSES[getColumnAlignment(type)]
