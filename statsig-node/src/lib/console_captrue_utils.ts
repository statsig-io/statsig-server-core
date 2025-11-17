/**
 * Utility functions for console capture payload parsing and processing
 */

export function safeStringify(
    val: unknown,
    maxKeysCount: number,
    maxDepth: number,
    maxLength: number,
  ): string {
    try {
      if (shouldNotStringify(val as object, maxKeysCount, maxDepth)) {
        return simpleStringify(val as object, maxLength);
      }
  
      if (typeof val === 'string') {
        return truncateString(val, maxLength);
      }
      if (typeof val === 'object' && val !== null) {
        return truncateString(JSON.stringify(val), maxLength);
      }
  
      return truncateString(String(val), maxLength);
    } catch {
      return truncateString('[Unserializable]', maxLength);
    }
  }
  
  export function shouldNotStringify(
    val: object,
    maxKeysCount: number,
    maxDepth: number,
  ): boolean {
    if (isPlainObject(val)) {
      if (Object.keys(val).length > maxKeysCount) {
        return true;
      }
      if (isObjectTooDeep(val, maxDepth)) {
        return true;
      }
    }
  
    if (typeof val === 'function') {
      return true;
    }
  
    return false;
  }
  
  export function isPlainObject(obj: unknown): boolean {
    return Object.prototype.toString.call(obj) === '[object Object]';
  }
  
  export function isObjectTooDeep(obj: unknown, maxDepth: number): boolean {
    if (maxDepth <= 0) {
      return true;
    }
    if (typeof obj !== 'object' || obj === null) {
      return false;
    }
    return Object.keys(obj).some((key) =>
      isObjectTooDeep(obj[key as keyof typeof obj], maxDepth - 1),
    );
  }
  
  export function getStackTrace(): string {
    const stack = new Error().stack;
    return stack ? stack.split('\n').slice(2, 5).join('\n') : '';
  }
  
  export function truncateString(str: string, maxLength: number): string {
    if (str.length <= maxLength) {
      return str;
    }
    return str.slice(0, maxLength) + '...';
  }
  
  export function simpleStringify(val: object, maxLength: number): string {
    return truncateString(val.toString(), maxLength);
  }