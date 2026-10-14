import test from 'node:test';
import assert from 'node:assert/strict';
import {filterDays} from '../src/filter.mjs';
const days=[
  {date:'2026-10-14',vehicle:{names:['Bike'],codes:['AB'],models:[{label:'MO-1'}]}},
  {date:'2026-10-13',vehicle:{names:['Bike'],codes:[],models:[{label:'MO-16'}]}},
  {date:'2025-01-01',vehicle:{names:[],codes:[],models:[]}},
];
test('model joins are exact, date bounds inclusive, metadata absence retained',()=>{
  assert.deepEqual(filterDays(days,{model:'MO-1'}),[days[0]]);
  assert.deepEqual(filterDays(days,{from:'2026-10-13',to:'2026-10-14'}),days.slice(0,2));
  assert.deepEqual(filterDays(days,{model:'unmatched'}),[days[2]]);
  assert.deepEqual(filterDays(days,{query:'ab'}),[days[0]]);
  assert.deepEqual(filterDays(days,{from:'2026-10-14',to:'2026-10-13'}),[]);
});
