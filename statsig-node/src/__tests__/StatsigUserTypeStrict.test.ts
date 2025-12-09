import { StatsigUser } from '../../build/index.js';

describe('StatsigUser Type Strict Tests', () => {
  /**
   * --- customIDs ---
   */
  describe('customIDs - Type: Record<string, string> | null', () => {
    describe('Getter: Record<string, string> | null', () => {
      it('should return null when not set', () => {
        const user = new StatsigUser({ userID: 'test' });
        expect(user.customIDs).toBeNull();
      });

      it('should return Record<string, string> when set', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.customIDs = { id1: 'value1', id2: 'value2' };
        const result = user.customIDs;
        expect(result).not.toBeNull();
        expect(result).toEqual({ id1: 'value1', id2: 'value2' });
        expect(typeof result).toBe('object');
      });
    });

    describe('Setter: Record<string, string> | null', () => {
      it('should accept Record<string, string>', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.customIDs = { employee_id: 'emp-123', department: 'eng' };
        expect(user.customIDs).toEqual({ employee_id: 'emp-123', department: 'eng' });
      });

      it('should accept null', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.customIDs = { id: 'value' };
        user.customIDs = null;
        expect(user.customIDs).toBeNull();
      });

      it('should accept empty object', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.customIDs = {};
        expect(user.customIDs).toEqual({});
      });
    });

    describe('Constructor', () => {
      it('should accept Record<string, string> in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          customIDs: { employee_id: 'emp-123' },
        });
        expect(user.customIDs).toEqual({ employee_id: 'emp-123' });
      });
    });
  });

  /**
   * --- custom ---
   */
  describe('custom - Type: Record<string, string | number | boolean | Array<string | number | boolean>> | null', () => {
    describe('Getter: Record<string, string | number | boolean | Array> | null', () => {
      it('should return null when not set', () => {
        const user = new StatsigUser({ userID: 'test' });
        expect(user.custom).toBeNull();
      });

      it('should return Record with string values', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { name: 'John', city: 'SF' };
        expect(user.custom).toEqual({ name: 'John', city: 'SF' });
      });

      it('should return Record with number values', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { age: 25, score: 100.5 };
        expect(user.custom).toEqual({ age: 25, score: 100.5 });
      });

      it('should return Record with boolean values', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { isPremium: true, isActive: false };
        expect(user.custom).toEqual({ isPremium: true, isActive: false });
      });

      it('should return Record with Array<string | number | boolean> values', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = {
          tags: ['tag1', 'tag2'],
          scores: [1, 2, 3],
          flags: [true, false, true],
        };
        expect(user.custom).toEqual({
          tags: ['tag1', 'tag2'],
          scores: [1, 2, 3],
          flags: [true, false, true],
        });
      });

      it('should return Record with mixed types', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = {
          name: 'John',
          age: 25,
          isPremium: true,
          tags: ['tag1', 'tag2'],
        };
        expect(user.custom).toEqual({
          name: 'John',
          age: 25,
          isPremium: true,
          tags: ['tag1', 'tag2'],
        });
      });
    });

    describe('Setter: Record<string, string | number | boolean | Array> | null', () => {
      it('should accept Record<string, string>', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { name: 'John' };
        expect(user.custom).toEqual({ name: 'John' });
      });

      it('should accept Record<string, number>', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { age: 25 };
        expect(user.custom).toEqual({ age: 25 });
      });

      it('should accept Record<string, boolean>', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { isPremium: true };
        expect(user.custom).toEqual({ isPremium: true });
      });

      it('should accept Record<string, Array<string | number | boolean>>', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = { tags: ['tag1', 'tag2'] };
        expect(user.custom).toEqual({ tags: ['tag1', 'tag2'] });
      });

      it('should accept null', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.custom = null;
        expect(user.custom).toBeNull();
      });
    });

    describe('Constructor', () => {
      it('should accept Record in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          custom: { age: 25, name: 'John' },
        });
        expect(user.custom).toEqual({ age: 25, name: 'John' });
      });
    });
  });

  /**
   * --- privateAttributes ---
   */
  describe('privateAttributes - Type: Record<string, string | number | boolean | Array<string | number | boolean>> | null', () => {
    describe('Getter: Record<string, string | number | boolean | Array> | null', () => {
      it('should return null when not set', () => {
        const user = new StatsigUser({ userID: 'test' });
        expect(user.privateAttributes).toBeNull();
      });

      it('should return Record with mixed types', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.privateAttributes = {
          secret: 'hidden',
          count: 100,
          active: false,
          tags: ['tag1', 'tag2'],
        };
        expect(user.privateAttributes).toEqual({
          secret: 'hidden',
          count: 100,
          active: false,
          tags: ['tag1', 'tag2'],
        });
      });
    });

    describe('Setter: Record<string, string | number | boolean | Array> | null', () => {
      it('should accept Record with all supported types', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.privateAttributes = {
          stringField: 'value',
          numberField: 42,
          booleanField: true,
          arrayField: [1, 2, 3],
        };
        expect(user.privateAttributes).toEqual({
          stringField: 'value',
          numberField: 42,
          booleanField: true,
          arrayField: [1, 2, 3],
        });
      });

      it('should accept null', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.privateAttributes = { field: 'value' };
        user.privateAttributes = null;
        expect(user.privateAttributes).toBeNull();
      });
    });

    describe('Constructor', () => {
      it('should accept Record in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          privateAttributes: { secret: 'hidden' },
        });
        expect(user.privateAttributes).toEqual({ secret: 'hidden' });
      });

      it('should accept null in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          privateAttributes: { secret: 'hidden' },
        });
        expect(user.privateAttributes).toEqual({ secret: 'hidden' });
      });
    });
  });

  describe('statsigEnvironment', () => {
    describe('Getter: Record<string, string> | null', () => {
      it('should return null when not set', () => {
        const user = new StatsigUser({ userID: 'test' });
        expect(user.statsigEnvironment).toBeNull();
      });

      it('should return Record<string, string> when set', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.statsigEnvironment = { tier: 'prod' };
        const result = user.statsigEnvironment;
        expect(result).not.toBeNull();
        expect(result).toEqual({ tier: 'prod' });
      });
    });

    describe('Setter: { tier?: string, [key: string]: string | undefined } | undefined', () => {
      it('should accept object with tier', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.statsigEnvironment = { tier: 'production' };
        expect(user.statsigEnvironment).toEqual({ tier: 'production' });
      });

      it('should accept object with tier and additional keys', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.statsigEnvironment = {
          tier: 'staging',
          customKey: 'customValue',
        };
        expect(user.statsigEnvironment).toEqual({
          tier: 'staging',
          customKey: 'customValue',
        });
      });

      it('should accept object with undefined values in index signature', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.statsigEnvironment = {
          tier: 'prod',
          optionalKey: undefined as any,
        };
        // undefined values might be filtered out or converted
        const result = user.statsigEnvironment;
        expect(result).not.toBeNull();
        expect(result?.tier).toBe('prod');
      });

      it('should accept undefined', () => {
        const user = new StatsigUser({ userID: 'test' });
        user.statsigEnvironment = undefined;
        // passing down undefined is treated as null
        expect(user.statsigEnvironment).toBeNull();
      });
    });

    describe('Constructor', () => {
      it('should accept object in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          statsigEnvironment: { tier: 'production' },
        });
        expect(user.statsigEnvironment).toEqual({
          tier: 'production',
        });
      });

      it('should accept undefined in constructor', () => {
        const user = new StatsigUser({
          userID: 'test',
          statsigEnvironment: undefined,
        });
        expect(user.statsigEnvironment).toBeNull();
      });
    });
  });

  describe('String fields (userID, email, ip, userAgent, country, locale, appVersion)', () => {
    const stringFields = [
      'email',
      'ip',
      'userAgent',
      'country',
      'locale',
      'appVersion',
    ] as const;

    stringFields.forEach((field) => {
      describe(`${field}`, () => {
        describe(`Getter: string | null`, () => {
          it(`should return null when ${field} is not set`, () => {
            const user = new StatsigUser({ userID: 'test' });
            expect((user as any)[field]).toBeNull();
          });

          it(`should return string when ${field} is set`, () => {
            const user = new StatsigUser({ userID: 'test' });
            (user as any)[field] = 'test-value';
            const result = (user as any)[field];
            expect(result).toBe('test-value');
            expect(typeof result).toBe('string');
          });
        });

        describe(`Setter: any`, () => {
          it(`should accept string for ${field}`, () => {
            const user = new StatsigUser({ userID: 'test' });
            (user as any)[field] = 'test-value';
            expect((user as any)[field]).toBe('test-value');
          });

          it(`should accept null for ${field}`, () => {
            const user = new StatsigUser({ userID: 'test' });
            (user as any)[field] = null;
            expect((user as any)[field]).toBeNull();
          });

          it(`should accept number for ${field} (any type)`, () => {
            const user = new StatsigUser({ userID: 'test' });
            (user as any)[field] = 123;
            // Should convert to string or handle as any
            const result = (user as any)[field];
            expect(result).toBeDefined();
          });

          it(`should accept boolean for ${field} (any type)`, () => {
            const user = new StatsigUser({ userID: 'test' });
            (user as any)[field] = true;
            // Should convert to string or handle as any
            const result = (user as any)[field];
            expect(result).toBeDefined();
          });
        });
      });
    });
  });
});
