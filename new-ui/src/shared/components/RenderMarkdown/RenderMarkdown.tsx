import type { HTMLProps } from 'react';
import './style.scss';
import clsx from 'clsx';
import ReactMarkdown from 'react-markdown';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';

const sanitizeSchema = {
  ...defaultSchema,
  tagNames: (defaultSchema.tagNames ?? []).filter((tag: string) => tag !== 'iframe'),
};

export const RenderMarkdown = ({
  content,
  containerProps,
}: {
  content?: string | null | undefined;
  containerProps?: HTMLProps<HTMLDivElement>;
}) => {
  const containerCustomClassName = containerProps?.className;

  return (
    <div
      {...containerProps}
      className={clsx('markdown-render', containerCustomClassName)}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeRaw, [rehypeSanitize, sanitizeSchema]]}
        components={{
          a: (props) => <a {...props} target="_blank" rel="noopener noreferrer" />,
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
};
