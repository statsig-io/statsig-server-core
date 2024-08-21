import test from 'ava'

import { statsigCreate } from '../bindings.js'

test('exports init function', (t) => {
  t.is(typeof statsigCreate, 'function')
})
