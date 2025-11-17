import * as consoleCaptureUtils from '../lib/console_captrue_utils';

describe('Console Capture Utils', () => {
  describe('safeStringify', () => {
    it('should stringify simple values', () => {
      expect(consoleCaptureUtils.safeStringify('hello', 10, 10, 1000)).toBe('hello');
      expect(consoleCaptureUtils.safeStringify(123, 10, 10, 1000)).toBe('123');
      expect(consoleCaptureUtils.safeStringify(true, 10, 10, 1000)).toBe('true');
    });

    it('should stringify objects', () => {
      const obj = { key: 'value', nested: { data: 'test' } };
      expect(consoleCaptureUtils.safeStringify(obj, 10, 10, 1000)).toBe('{"key":"value","nested":{"data":"test"}}');
    });

    it('should truncate long strings', () => {
      const longString = 'a'.repeat(2000);
      const result = consoleCaptureUtils.safeStringify(longString, 10, 10, 100);
      expect(result).toBe('a'.repeat(100) + '...');
    });

    it('should handle circular objects by returning [Unserializable]', () => {
      const circularObj: any = {};
      circularObj.self = circularObj;
      
      const result = consoleCaptureUtils.safeStringify(circularObj, 10, 10, 1000);
      expect(result).toBe('[Unserializable]');
    });

    it('should handle functions', () => {
      const func = () => 'test';
      const result = consoleCaptureUtils.safeStringify(func, 10, 10, 1000);
      expect(result).toBe(func.toString());
    });

    it('should handle objects that are too deep', () => {
        let deepObj: any = {};
        let current = deepObj;
        
        // Create 1000 levels deep (exceeds maxDepth of 10)
        for (let i = 0; i < 1000; i++) {
          current.nested = {};
          current = current.nested;
        }
        
        expect(() => {
          const result = consoleCaptureUtils.safeStringify(deepObj, 10, 10, 1000);
          expect(result).toBe('[object Object]');
        }).not.toThrow();
      });
  
    it('should handle objects with too many keys', () => {
    const objWithManyKeys: any = {};
    
    // Create object with 1000 keys (exceeds maxKeysCount of 10)
    for (let i = 0; i < 1000; i++) {
        objWithManyKeys[`key${i}`] = `value${i}`;
    }
    
    expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithManyKeys, 10, 10, 1000);
        expect(result).toBe('[object Object]');
    }).not.toThrow();
    });
  });

  describe('shouldNotStringify', () => {
    it('should return false for simple objects', () => {
      expect(consoleCaptureUtils.shouldNotStringify({ key: 'value' }, 10, 10)).toBe(false);
    });

    it('should return true for objects with too many keys', () => {
      const obj: any = {};
      for (let i = 0; i < 15; i++) {
        obj[`key${i}`] = `value${i}`;
      }
      expect(consoleCaptureUtils.shouldNotStringify(obj, 10, 10)).toBe(true);
    });

    it('should return true for objects that are too deep', () => {
      let obj: any = {};
      let current = obj;
      for (let i = 0; i < 15; i++) {
        current.nested = {};
        current = current.nested;
      }
      expect(consoleCaptureUtils.shouldNotStringify(obj, 10, 10)).toBe(true);
    });

    it('should return true for functions', () => {
      expect(consoleCaptureUtils.shouldNotStringify(() => {}, 10, 10)).toBe(true);
    });
  });

  describe('isPlainObject', () => {
    it('should return true for plain objects', () => {
      expect(consoleCaptureUtils.isPlainObject({})).toBe(true);
      expect(consoleCaptureUtils.isPlainObject({ key: 'value' })).toBe(true);
    });

    it('should return false for arrays', () => {
      expect(consoleCaptureUtils.isPlainObject([])).toBe(false);
    });

    it('should return false for null', () => {
      expect(consoleCaptureUtils.isPlainObject(null)).toBe(false);
    });

    it('should return false for primitives', () => {
      expect(consoleCaptureUtils.isPlainObject('string')).toBe(false);
      expect(consoleCaptureUtils.isPlainObject(123)).toBe(false);
    });
  });

  describe('isObjectTooDeep', () => {
    it('should return false for simple objects', () => {
      expect(consoleCaptureUtils.isObjectTooDeep({ key: 'value' }, 5)).toBe(false);
    });

    it('should return true when maxDepth is 0', () => {
      expect(consoleCaptureUtils.isObjectTooDeep({}, 0)).toBe(true);
    });

    it('should detect deeply nested objects', () => {
      let obj: any = {};
      let current = obj;
      for (let i = 0; i < 5; i++) {
        current.nested = {};
        current = current.nested;
      }
      expect(consoleCaptureUtils.isObjectTooDeep(obj, 3)).toBe(true);
    });

    it('should return false for circular objects', () => {
    // circular objects should be [unserializable] by being caught in the json.stringify try catch
      const circularObj: any = {};
      circularObj.self = circularObj;
      expect(consoleCaptureUtils.isObjectTooDeep(circularObj, 5)).toBe(false);
    });
  });

  describe('getStackTrace', () => {
    it('should return a stack trace string', () => {
      const stack = consoleCaptureUtils.getStackTrace();
      expect(typeof stack).toBe('string');
      expect(stack).toContain('ConsoleCaptureUtils.test.ts');
    });
  });

  describe('truncateString', () => {
    it('should return original string if within limit', () => {
      const str = 'short string';
      expect(consoleCaptureUtils.truncateString(str, 100)).toBe(str);
    });

    it('should truncate long strings', () => {
      const str = 'a'.repeat(200);
      const result = consoleCaptureUtils.truncateString(str, 100);
      expect(result).toBe('a'.repeat(100) + '...');
    });
  });

  describe('simpleStringify', () => {
    it('should call toString and truncate', () => {
      const obj = { toString: () => 'test object' };
      const result = consoleCaptureUtils.simpleStringify(obj, 100);
      expect(result).toBe('test object');
    });
  });

  describe('wrapFunctionWithRestore', () => {
    it('should wrap a function and provide restore capability', () => {
      const target = { testFn: jest.fn() };
      const originalFn = target.testFn;
      
      const restoreFn = consoleCaptureUtils.wrapFunctionWithRestore(
        target,
        'testFn',
        (original) => (...args) => {
          return original(...args);
        }
      );
      
      expect(typeof restoreFn).toBe('function');
      expect(target.testFn).not.toBe(originalFn);
      
      target.testFn('test');
      expect(originalFn).toHaveBeenCalledWith('test');
      
      restoreFn();
      expect(target.testFn).toBe(originalFn);
    });

    it('should handle non-function targets gracefully', () => {
      const target = { testProp: 'not a function' };
      
      const restoreFn = consoleCaptureUtils.wrapFunctionWithRestore(
        target,
        'testProp',
        (original) => original
      );
      
      expect(typeof restoreFn).toBe('function');
      restoreFn(); // Should not throw
    });

    it('should handle wrapping errors gracefully', () => {
      const target = { testFn: jest.fn() };
      
      const restoreFn = consoleCaptureUtils.wrapFunctionWithRestore(
        target,
        'testFn',
        () => {
          throw new Error('Wrapper failed');
        }
      );
      
      expect(typeof restoreFn).toBe('function');
      restoreFn(); // Should not throw
    });
  });

  describe('Edge Case Object Stringification', () => {
    it('should handle deeply nested circular references', () => {
      const obj1: any = { name: 'obj1' };
      const obj2: any = { name: 'obj2', ref1: obj1 };
      const obj3: any = { name: 'obj3', ref2: obj2 };
      obj1.ref3 = obj3;
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(obj1, 10, 10, 1000);
        expect(result).toBe('[Unserializable]');
      }).not.toThrow();
    });

    it('should handle objects with getters that throw errors', () => {
      const objWithThrowingGetter = {
        normalProp: 'value',
        get throwingProp() {
          throw new Error('Getter error');
        }
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithThrowingGetter, 10, 10, 1000);
        expect(result).toBe('[Unserializable]');
      }).not.toThrow();
    });

    it('should handle objects with very large arrays', () => {
      const largeArray = new Array(100000).fill('item');
      const objWithLargeArray = { data: largeArray };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithLargeArray, 10, 10, 1000);
        expect(result).toContain('...');
      }).not.toThrow();
    });

    it('should handle objects with special values', () => {
      const specialValues = {
        undefined: undefined,
        null: null,
        nan: NaN,
        infinity: Infinity,
        negativeInfinity: -Infinity,
        symbol: Symbol('test'),
        bigint: BigInt(123),
        date: new Date(),
        regex: /test/gi,
        error: new Error('test error'),
        map: new Map([['key', 'value']]),
        set: new Set([1, 2, 3]),
        weakMap: new WeakMap(),
        weakSet: new WeakSet(),
        arrayBuffer: new ArrayBuffer(8),
        typedArray: new Uint8Array([1, 2, 3])
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(specialValues, 20, 10, 1000);
        expect(typeof result).toBe('string');
        expect(result).toContain('[Unserializable]');
      }).not.toThrow();
    });

    it('should handle objects with prototype pollution attempts', () => {
      const maliciousObj = {
        __proto__: { isAdmin: true },
        constructor: { prototype: { isAdmin: true } },
        toString: () => 'malicious'
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(maliciousObj, 10, 10, 1000);
        expect(typeof result).toBe('string');
      }).not.toThrow();
    });

    it('should handle objects with functions that have circular references', () => {
      const funcObj: any = function() {};
      funcObj.self = funcObj;
      funcObj.nested = { parent: funcObj };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(funcObj, 10, 10, 1000);
        expect(result).toBe(funcObj.toString());
      }).not.toThrow();
    });

    it('should handle objects with toString that throws', () => {
      const objWithThrowingToString = {
        value: {a: 'b', c: 'd', e: 'f'},
        toString() {
          throw new Error('toString error');
        }
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithThrowingToString, 1, 10, 1000); // exceed maxKeysCount so it calls toString()
        expect(result).toBe('[Unserializable]');
      }).not.toThrow();
    });

    it('should handle objects with valueOf that throws', () => {
      const objWithThrowingValueOf = {
        value: 'test',
        valueOf() {
          throw new Error('valueOf error');
        }
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithThrowingValueOf, 10, 10, 1000);
        expect(result).toBe(JSON.stringify(objWithThrowingValueOf));
      }).not.toThrow();
    });

    it('should handle objects with enumerable properties that throw on access', () => {
      const objWithThrowingProperties = {};
      
      Object.defineProperty(objWithThrowingProperties, 'throwingProp', {
        enumerable: true,
        get() {
          throw new Error('Property access error');
        }
      });
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithThrowingProperties, 10, 10, 1000);
        expect(result).toBe('[Unserializable]');
      }).not.toThrow();
    });

    it('should handle objects with non-enumerable properties that throw', () => {
      const objWithNonEnumerableThrowing = {};
      
      Object.defineProperty(objWithNonEnumerableThrowing, 'throwingProp', {
        enumerable: false,
        get() {
          throw new Error('Non-enumerable property error');
        }
      });
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(objWithNonEnumerableThrowing, 10, 10, 1000);
        expect(typeof result).toBe('string');
      }).not.toThrow();
    });

    it('should handle very large strings without crashing', () => {
      const veryLongString = 'a'.repeat(1000000);
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(veryLongString, 10, 10, 1000);
        expect(result).toContain('...');
        expect(result.length).toBe(1003); // 1000 + '...'
      }).not.toThrow();
    });

    it('should handle objects with mixed data types and edge cases', () => {
      const mixedObj = {
        string: 'test',
        number: 123,
        boolean: true,
        null: null,
        undefined: undefined,
        array: [1, 2, 3],
        nested: {
          deep: {
            deeper: {
              value: 'deep value'
            }
          }
        },
        func: () => 'test',
        symbol: Symbol('test'),
        date: new Date(),
        regex: /test/gi,
        error: new Error('test'),
        map: new Map(),
        set: new Set(),
        weakMap: new WeakMap(),
        weakSet: new WeakSet(),
        arrayBuffer: new ArrayBuffer(8),
        typedArray: new Uint8Array([1, 2, 3]),
        promise: Promise.resolve('test'),
        proxy: new Proxy({}, {}),
        generator: function* () { yield 1; },
        asyncFunc: async () => 'test'
      };
      
      expect(() => {
        const result = consoleCaptureUtils.safeStringify(mixedObj, 100, 10, 1000);
        expect(typeof result).toBe('string');
        expect(result).toContain(JSON.stringify(mixedObj));
      }).not.toThrow();
    });
  });
});
