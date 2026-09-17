import type { Locale } from './content';

// Toggle the leading `/zh` prefix to switch locales while keeping the
// current page path. e.g. '/gyid' <-> '/zh/gyid', '/' <-> '/zh'.
export function swapLocale(path: string, target: Locale): string {
  const clean = path.replace(/\/+$/, '') || '/';
  if (target === 'zh') {
    return clean === '/' ? '/zh' : '/zh' + clean;
  }
  // target en
  if (clean === '/zh') return '/';
  const stripped = clean.replace(/^\/zh/, '');
  return stripped === '' ? '/' : stripped;
}
