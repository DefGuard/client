import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { error } from '@tauri-apps/plugin-log';
import { type PropsWithChildren, useCallback, useEffect, useRef } from 'react';
import z, { string } from 'zod';
import useAddInstance from '../../hooks/useAddInstance';

enum DeepLink {
  AddInstance = 'addinstance',
}

export const linkStorageKey = 'lastSuccessfullyHandledDeepLink';

export const storeLink = (value: string) => {
  sessionStorage.setItem(linkStorageKey, value);
};

const readStoreLink = (): string | null => {
  return sessionStorage.getItem(linkStorageKey);
};

const addInstanceLinkSchema = z.object({
  token: string().trim().min(1),
  url: string().trim().min(1).url(),
});

const AddInstanceLink = z.object({
  link: z.literal(DeepLink.AddInstance),
  data: addInstanceLinkSchema,
});

const validLinkPayload = z.discriminatedUnion('link', [AddInstanceLink]);

type LinkPayload = z.infer<typeof validLinkPayload>;

const linkIntoPayload = (link: URL | null): LinkPayload | null => {
  if (link == null) return null;

  const searchData = Object.fromEntries(new URLSearchParams(link.search));
  const linkKey = [link.hostname, link.pathname]
    .map((l) => l.trim().replaceAll('/', ''))
    .filter((l) => l !== '')[0] as string;
  const payload = {
    link: linkKey,
    data: searchData,
  };
  const result = validLinkPayload.safeParse(payload);
  if (result.success) {
    return result.data;
  } else {
    error(`Link ${link} was rejected due to schema validation.`);
  }
  return null;
};

export const DeepLinkProvider = ({ children }: PropsWithChildren) => {
  const mounted = useRef(false);

  const { handleAddInstance } = useAddInstance();

  const handleValidLink = useCallback(
    async (payload: LinkPayload, rawLink?: string) => {
      const { data, link } = payload;
      switch (link) {
        case DeepLink.AddInstance:
          await handleAddInstance(data, rawLink);
          break;
      }
      if (rawLink) {
        storeLink(rawLink);
      }
    },
    [handleAddInstance],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: only on mount
  useEffect(() => {
    if (!mounted.current) {
      mounted.current = true;

      let unlisten: (() => void) | undefined;
      (async () => {
        const start = await getCurrent();
        if (start != null) {
          const lastLink = readStoreLink();
          // if the link is exact as last successfully executed link
          // this is only necessary bcs in dev mode window is hot reloaded causing the startup link to be handled multiple times over.
          if (lastLink != null && lastLink === start[0]) {
            return;
          }
          const payload = linkIntoPayload(new URL(start[0]));
          if (payload != null) {
            try {
              handleValidLink(payload, start[0]);
            } catch (e) {
              error(
                `Failed to handle valid deep link ${payload.link}!\n${JSON.stringify(e)}`,
              );
            }
          }
        }
        unlisten = await onOpenUrl((urls) => {
          if (urls?.length) {
            const link = urls[0];
            const payload = linkIntoPayload(new URL(link));
            if (payload != null) {
              try {
                handleValidLink(payload);
              } catch (e) {
                error(`Failed to handle valid deep link ${payload?.link} action!`);
                error(JSON.stringify(e));
              }
            }
          }
        });
      })();
      return () => {
        unlisten?.();
      };
    }
  }, []);

  return <>{children}</>;
};
