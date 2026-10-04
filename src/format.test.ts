import { test } from 'node:test';
import assert from 'node:assert/strict';
import { duration, percent, resetTime, selectedBucket, resetCount, resetExpiryRows } from './format.ts';
test('reset expiry distinguishes absent details, unlimited validity, and partial lists', () => {
  assert.deepEqual(resetExpiryRows(1, null), ['사용기한 정보 없음']);
  assert.deepEqual(resetExpiryRows(1, [null]), ['사용기한 없음']);
  assert.deepEqual(resetExpiryRows(0, []), ['사용 가능한 초기화가 없습니다.']);
  assert.deepEqual(resetExpiryRows(3, [null]), ['초기화 1: 사용기한 없음', '나머지 2회: 사용기한 정보 없음']);
  assert.match(resetExpiryRows(1, [1730947200])[0], /2024.*까지 사용 가능/);
});
test('remaining reset count preserves zero and never invents missing values', () => {
  assert.equal(resetCount(7), '7회');
  assert.equal(resetCount(0), '0회');
  for (const value of [null, undefined, -1, 1.5, NaN, Infinity]) assert.equal(resetCount(value), '정보 없음');
});
test('missing usage is unknown, never zero', () => {
  assert.equal(percent(null), '정보 없음');
  assert.equal(percent(25), '25.0%');
});
test('server window length is preserved', () => {
  assert.equal(duration(15), '15분');
  assert.equal(duration(300), '5시간');
  assert.equal(duration(null), '기간 정보 없음');
});
test('missing reset time stays unknown', () => {
  assert.equal(resetTime(null), '초기화 시각 정보 없음');
});
test('removed bucket selection falls back to an existing bucket', () => {
  const buckets = [{ id: 'a' }, { id: 'b' }];
  assert.equal(selectedBucket(buckets, 'gone')?.id, 'a');
  assert.equal(selectedBucket(buckets, 'b')?.id, 'b');
  assert.equal(selectedBucket([], 'b'), undefined);
});
