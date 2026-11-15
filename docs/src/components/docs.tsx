import { type SerializedPageTree, useFumadocsLoader } from 'fumadocs-core/source/client';
import { usePathname } from 'fumadocs-core/framework';
import { useFooterItems } from 'fumadocs-ui/utils/use-footer-items';
import type { TOCItemType } from 'fumadocs-core/toc';
import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { Container as DocsContainer } from 'fumadocs-ui/layouts/docs/slots/container';
import { Children, type ComponentProps } from 'react';
import {
  DocsDescription,
  DocsPage,
  DocsTitle,
  MarkdownCopyButton,
  PageFooter,
  type FooterProps,
} from 'fumadocs-ui/layouts/docs/page';
import { Provider, type ProviderProps } from '@/components/provider';
import { baseOptions } from '@/lib/layout.shared';

export interface DocsProps extends ProviderProps {
  tree: SerializedPageTree;
  title: string;
  description?: string;
  toc: TOCItemType[];
  markdownUrl: string;
  home?: boolean;
}

function ReaderFooter(props: FooterProps) {
  const pathname = usePathname();
  const pages = useFooterItems();
  if (!pathname.startsWith('/docs/library')) return <PageFooter {...props} data-page-chrome="" />;
  const current = pages.findIndex((page) => page.url.replace(/\/$/, '') === pathname.replace(/\/$/, ''));
  const items = props.items ?? (current < 0 ? {} : { previous: pages[current - 1], next: pages[current + 1] });
  const keep = (item: NonNullable<FooterProps['items']>['next']) =>
    item && !/^\/docs\/standard(?:\/|$)/.test(item.url) ? item : undefined;
  return <PageFooter {...props} data-page-chrome="" items={{ previous: keep(items.previous), next: keep(items.next) }} />;
}

function HomeDocsContainer({ children, ...props }: ComponentProps<typeof DocsContainer>) {
  // The default container receives the skip-to-content hash link first. This
  // short, single-screen page has no hash navigation; keep all docs chrome.
  const [, ...content] = Children.toArray(children);
  return <DocsContainer {...props}>{content}</DocsContainer>;
}

/**
 * The React island of docs pages, the content is passed as children.
 */
export function Docs({
  tree,
  title,
  description,
  toc,
  markdownUrl,
  home = false,
  children,
  ...props
}: DocsProps) {
  const data = useFumadocsLoader({ tree });

  return (
    <Provider {...props}>
      <DocsLayout {...baseOptions()} tree={data.tree} slots={home ? { container: HomeDocsContainer } : undefined}>
        {home ? <DocsPage full toc={[]} breadcrumb={{ enabled: false }} footer={{ enabled: false }}
          tableOfContent={{ enabled: false }} tableOfContentPopover={{ enabled: false }} className="home-doc-page">
          {children}
        </DocsPage> : <DocsPage toc={toc} slots={{ footer: ReaderFooter }}>
          <DocsTitle>{title}</DocsTitle>
          <DocsDescription>{description}</DocsDescription>
          <div className="flex flex-row gap-2 items-center border-b -mt-4 pb-6">
            <MarkdownCopyButton markdownUrl={markdownUrl} />
          </div>
          {children}
        </DocsPage>}
      </DocsLayout>
    </Provider>
  );
}
