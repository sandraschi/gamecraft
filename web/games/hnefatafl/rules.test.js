import { test, expect } from 'bun:test';
import { ruleChecks } from './rulecheck.js';

for (const c of ruleChecks()) {
  test(c.name, () => {
    expect(c.detail).toBe('');
    expect(c.pass).toBe(true);
  });
}
