/**
 * Utility functions for console capture payload parsing and processing
 */

export function safeStringify(
  val: unknown,
  maxKeysCount: number,
  maxDepth: number,
  maxLength: number
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
  maxDepth: number
): boolean {
  if (isPlainObject(val)) {
    if (Object.keys(val).length > maxKeysCount) {
      return true;
    }
    if (isObjectTooDeep(val, maxDepth)) {
      return true;
    }

    return false;
  }

  if (typeof val === 'function') {
    return true;
  }

  return false;
}

export function isPlainObject(obj: unknown): boolean {
  return Object.prototype.toString.call(obj) === '[object Object]';
}

export function isObjectTooDeep(
  obj: unknown,
  maxDepth: number,
  seen: WeakSet<object> = new WeakSet()
): boolean {
  if (maxDepth <= 0) {
    return true;
  }

  if (typeof obj !== 'object' || obj === null) {
    return false; // primitives are never "too deep"
  }

  if (seen.has(obj)) {
    return false; // cycle detected
  }

  seen.add(obj);

  return Object.values(obj).some((value) =>
    isObjectTooDeep(value, maxDepth - 1, seen)
  );
}

export function getStackTrace(): string | null {
  const stack = new Error().stack;
  return stack ? stack.split('\n').slice(2, 5).join('\n') : null;
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

export function wrapFunctionWithRestore(
  targetObject: Record<string, unknown>,
  functionName: string,
  wrapperFactory: (
    original: (...args: unknown[]) => unknown
  ) => (...args: unknown[]) => unknown
): () => void {
  const originalFunction = targetObject[functionName];

  if (typeof originalFunction !== 'function') {
    return () => {
      // noop
    };
  }

  try {
    const wrappedFunction = wrapperFactory(
      originalFunction as (...args: unknown[]) => void
    );

    Object.defineProperty(wrappedFunction, '__statsig_original__', {
      enumerable: false,
      value: originalFunction,
    });

    targetObject[functionName] = wrappedFunction;

    // Restore function
    return () => {
      targetObject[functionName] = originalFunction;
    };
  } catch {
    return () => {
      // noop
    };
  }
}
