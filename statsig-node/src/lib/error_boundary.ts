export class ErrorBoundary {
  static wrap(instance: unknown): void {
    try {
      const obj = instance as Record<string, unknown>;

      _getAllInstanceMethodNames(obj).forEach((name) => {
        const original = obj[name] as (...args: unknown[]) => unknown;
        if ('$EB' in original) {
          return;
        }

        obj[name] = (...args: unknown[]) => {
          return this._capture(name, () => original.apply(instance, args));
        };
        (obj[name] as { $EB: boolean }).$EB = true;
      });
    } catch (err) {
      this._onError('eb:wrap', err);
    }
  }

  private static _capture(tag: string, task: () => unknown): unknown {
    try {
      const res = task();
      if (res && res instanceof Promise) {
        return res.catch((err) => this._onError(tag, err));
      }
      return res;
    } catch (error) {
      this._onError(tag, error);
      return null;
    }
  }

  private static _onError(tag: string, error: unknown): void {
    console.error(tag, error);
  }
}

function _getAllInstanceMethodNames(
  instance: Record<string, unknown>,
): string[] {
  const names = new Set<string>();

  let proto = Object.getPrototypeOf(instance) as Record<string, unknown>;
  while (proto && proto !== Object.prototype) {
    Object.getOwnPropertyNames(proto)
      .filter((prop) => typeof proto?.[prop] === 'function')
      .forEach((name) => names.add(name));
    proto = Object.getPrototypeOf(proto) as Record<string, unknown>;
  }

  return Array.from(names);
}
