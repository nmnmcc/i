import { type SerializedPageTree, useFumadocsLoader } from 'fumadocs-core/source/client';
import { usePathname } from 'fumadocs-core/framework';
import { useFooterItems } from 'fumadocs-ui/utils/use-footer-items';
import type { TOCItemType } from 'fumadocs-core/toc';
import { DocsLayout } from 'fumadocs-ui/layouts/docs';
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
}

function ReaderFooter(props: FooterProps) {
  const pathname = usePathname();
  const pages = useFooterItems();
  if (!pathname.startsWith('/docs/library')) return <PageFooter {...props} />;
  const current = pages.findIndex((page) => page.url.replace(/\/$/, '') === pathname.replace(/\/$/, ''));
  const items = props.items ?? (current < 0 ? {} : { previous: pages[current - 1], next: pages[current + 1] });
  const keep = (item: NonNullable<FooterProps['items']>['next']) =>
    item && !/^\/docs\/standard(?:\/|$)/.test(item.url) ? item : undefined;
  return <PageFooter {...props} items={{ previous: keep(items.previous), next: keep(items.next) }} />;
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
  children,
  ...props
}: DocsProps) {
  const data = useFumadocsLoader({ tree });

  return (
    <Provider {...props}>
      <DocsLayout {...baseOptions()} tree={data.tree}>
        <DocsPage toc={toc} slots={{ footer: ReaderFooter }}>
          <DocsTitle>{title}</DocsTitle>
          <DocsDescription>{description}</DocsDescription>
          <div className="flex flex-row gap-2 items-center border-b -mt-4 pb-6">
            <MarkdownCopyButton markdownUrl={markdownUrl} />
          </div>
          {children}
        </DocsPage>
      </DocsLayout>
    </Provider>
  );
}
