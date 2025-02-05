import { round } from 'lodash';

import { mmToKm, mmToM } from './physics';
import type { Unit } from './types';

export const NARROW_NO_BREAK_SPACE = '\u202f';
export const NO_BREAK_SPACE = '\xa0';

export function conditionalStringConcat<Condition>(
  elements: [Condition, string][],
  separator = ', '
) {
  function elementString([element, name]: [Condition, string]) {
    return element ? [name] : [];
  }
  return elements.reduce<string[]>((acc, el) => [...acc, ...elementString(el)], []).join(separator);
}

export function formatKmValue(value: number, unit: Unit = 'meters', digits = 3) {
  let divider = 1000;
  if (unit === 'millimeters') {
    divider = 1000000;
  }
  return `${(value / divider).toFixed(digits)}${NO_BREAK_SPACE}km`;
}

export function language2flag(lng: string) {
  switch (lng) {
    case 'en':
      return 'gb';
    case 'uk':
      return 'ua';
    default:
      return lng;
  }
}

export function snakeToCamel(str: string) {
  return str.replace(/[^a-zA-Z0-9]+(.)/g, (_, chr: string) => chr.toUpperCase());
}

export function getTranslationKey(translationList: string | undefined, item: string): string {
  return `${translationList ? `${translationList}.` : ''}${item}`;
}

export function geti18nKeyForNull(str: string | null): string {
  return str || `N/C`;
}

/** Given a string, return a number or undefined */
export function parseNumber(str: string) {
  return str && !Number.isNaN(Number(str)) ? Number(str) : undefined;
}

/**
 * Given a string, return a number or 0
 * Useful for number input
 */
export function convertInputStringToNumber(str: string) {
  return parseNumber(str) || 0;
}

/**
 * Given an UIC number, check if it begins with 87,
 * if true return the UIC without the 87,
 * if false return the full UIC
 * @param uic full UIC and CI code (8 digits)
 */
export function formatUicToCi(uic: number) {
  return uic.toString().replace(/^87/, '');
}

/**
 * Normalizes a string by removing accents and converting to lowercase.
 *
 * Accent removal is done using `String.normalize` which converts accented characters to their
 * decomposed form (e.g. é -> e + ´). We then remove all characters in the range of U+0300 to U+036f
 * which contains all unicode combining diacritical marks (https://en.wikipedia.org/wiki/Combining_Diacritical_Marks).
 */
export function normalized(s: string): string {
  return s
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '');
}

export const TEXT_AREA_MAX_LENGTH = 4096;
export const SMALL_TEXT_AREA_MAX_LENGTH = 1024;
export const TEXT_INPUT_MAX_LENGTH = 255;
export const SMALL_INPUT_MAX_LENGTH = 128;

export const isInvalidString = (maxLengthAuthorized: number, field?: string | null): boolean =>
  !!field && field.length > maxLengthAuthorized;

/**
 * Given a distance in millimeters, returns a human readable string.
 * Examples:
 * - 927520 -> 927m
 * - 1927520 -> 1.92Km
 *
 * @param distance the distance in millimeters
 * @returns A human readable string of the distance
 */
export function distanceToHumanReadable(distance: number): string {
  if (!distance) return '0';

  const kms = mmToKm(distance);
  if (Math.floor(kms) > 0) return `${round(kms, 2)}km`;

  const meters = mmToM(distance);
  if (Math.floor(meters) > 0) return `${round(meters, 0)}m`;

  return `${distance}mm`;
}

export function capitalizeFirstLetter(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1).toLowerCase();
}
