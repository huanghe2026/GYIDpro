/**
 * 页面注册表：slug → 页面组件。
 *
 * 新增页面只需在这里加一行，英文 `/…` 与中文 `/zh/…` 会同时生成，
 * 避免两种语言路由漂移。
 */
import type { Copy } from '../i18n/ui';

import Home from './pages/Home.astro';
import Protocol from './pages/Protocol.astro';
import Architecture from './pages/Architecture.astro';
import About from './pages/About.astro';
import DocsIndex from './pages/DocsIndex.astro';
import DocProtocolNotes from './pages/DocProtocolNotes.astro';
import DocCalibration from './pages/DocCalibration.astro';
import DocApi from './pages/DocApi.astro';
import ConsoleIdentity from './pages/ConsoleIdentity.astro';
import ConsoleCollect from './pages/ConsoleCollect.astro';
import ConsoleImport from './pages/ConsoleImport.astro';
import ConsoleVerify from './pages/ConsoleVerify.astro';

export const pageRegistry = {
  '': Home,
  protocol: Protocol,
  architecture: Architecture,
  about: About,
  docs: DocsIndex,
  'docs/protocol-notes': DocProtocolNotes,
  'docs/calibration': DocCalibration,
  'docs/api': DocApi,
  console: ConsoleIdentity,
  'console/collect': ConsoleCollect,
  'console/import': ConsoleImport,
  'console/verify': ConsoleVerify,
};

export type PageSlug = keyof typeof pageRegistry;

/** 每个 slug 的页面标题/描述（用于 <title> 与 meta）。 */
export function pageMeta(slug: PageSlug, t: Copy): { title?: string; description?: string } {
  const map: Partial<Record<PageSlug, { title: string; description?: string }>> = {
    protocol: { title: t.protocol.title, description: t.protocol.lead },
    architecture: { title: t.architecture.title, description: t.architecture.lead },
    about: { title: t.about.title, description: t.about.lead },
    docs: { title: t.docs.indexTitle, description: t.docs.indexLead },
    'docs/protocol-notes': {
      title: t.docs.cards[0].title,
      description: t.docs.cards[0].desc,
    },
    'docs/calibration': { title: t.docs.cards[1].title, description: t.docs.cards[1].desc },
    'docs/api': { title: t.docs.apiTitle, description: t.docs.apiLead },
    console: { title: t.console.title, description: t.console.lead },
    'console/collect': { title: `${t.console.title} · ${t.console.tabs.collect}` },
    'console/import': { title: `${t.console.title} · ${t.console.tabs.import}` },
    'console/verify': { title: `${t.console.title} · ${t.console.tabs.verify}` },
  };
  return map[slug] ?? {};
}
