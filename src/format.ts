export function percent(value: number | null | undefined): string {
  return value == null || !Number.isFinite(value) ? '정보 없음' : `${value.toFixed(1)}%`;
}
export function resetCount(value: number | null | undefined): string {
  return value == null || !Number.isSafeInteger(value) || value < 0 ? '정보 없음' : `${value.toLocaleString('ko-KR')}회`;
}
export function resetExpiryRows(count: number | null | undefined, expirations: (number | null)[] | null | undefined): string[] {
  if (count === 0) return ['사용 가능한 초기화가 없습니다.'];
  if (count == null || !Number.isSafeInteger(count) || count < 0 || !expirations?.length) return ['사용기한 정보 없음'];
  const known = expirations.slice(0, count);
  const rows = known.map((seconds, index) => {
    const date = seconds == null ? null : new Date(seconds * 1000);
    const expiry = seconds === null ? '사용기한 없음' : date && Number.isFinite(date.getTime()) ? `${date.toLocaleString('ko-KR')}까지 사용 가능` : '사용기한 정보 없음';
    return count === 1 ? expiry : `초기화 ${index + 1}: ${expiry}`;
  });
  if (known.length < count) rows.push(`나머지 ${count - known.length}회: 사용기한 정보 없음`);
  return rows;
}
export function duration(mins: number | null | undefined): string {
  if (mins == null) return '기간 정보 없음';
  if (mins % 1440 === 0 && mins > 0) return `${mins / 1440}일`;
  if (mins % 60 === 0 && mins > 0) return `${mins / 60}시간`;
  return `${mins}분`;
}
export function resetTime(seconds: number | null | undefined): string {
  if (seconds == null) return '초기화 시각 정보 없음';
  return new Date(seconds * 1000).toLocaleString('ko-KR', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}
export function selectedBucket<T extends { id: string }>(buckets: T[], id: string | null | undefined): T | undefined {
  return buckets.find(b => b.id === id) ?? buckets[0];
}
