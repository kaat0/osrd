/** Returns a value clamped within the inclusive range [min, max] */
export function clamp(value: number, [min, max]: [number, number]) {
  if (value >= max) return max;
  if (value <= min) return min;
  return value;
}

export function budgetFormat(amount: number | bigint) {
  const amountFormatted = new Intl.NumberFormat('fr-FR', {
    style: 'currency',
    currency: 'EUR',
    minimumFractionDigits: 0,
  }).format(amount);
  return amountFormatted;
}

// This function takes a train duration & the distributed intervals, and return the position of train inside intervals
export function valueToInterval(value?: number, intervals?: number[]) {
  if (value && intervals) {
    if (value < intervals[1]) return 0;
    if (value < intervals[2]) return 1;
    return 2;
  }
  return undefined;
}

export function isFloat(n: number) {
  return Number(n) === n && n % 1 !== 0;
}
/**
 * If it a float number it will strip the decimal digits
 * @param value value to strip
 * @param decimalPlaces number of decimal places to keep
 * @returns stripped number
 */
export function stripDecimalDigits(value: number, decimalPlaces: number): number {
  if (!isFloat(value) || !Number.isInteger(decimalPlaces) || decimalPlaces < 0) {
    return value;
  }

  const parts = value.toString().split('.');

  if (parts.length !== 2) {
    return value;
  }

  const stripedDecimal = parts[1].substring(0, decimalPlaces);
  const result = Number(`${parts[0]}.${stripedDecimal}`);
  return result;
}

/**
 * Checks if a floating-point number has more decimal places than specified.
 * @param value the floating-point number to check.
 * @param numberOfDecimal the maximum allowed number of decimal places.
 * @returns true if the number has more decimal places than allowed
 */
export const isInvalidFloatNumber = (value: number, numberOfDecimal: number): boolean => {
  if (!isFloat(value)) return false;
  const stringifyValue = value.toString();
  return stringifyValue.split('.')[1].length > numberOfDecimal;
};
