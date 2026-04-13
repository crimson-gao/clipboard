import type { Category } from './clip-utils';

type CategoryCounts = {
  text: number;
  image: number;
  file: number;
  favorite: number;
};

export function buildCategories(counts: CategoryCounts): Category[] {
  return [
    {
      key: 'all',
      label: '全部',
      icon: (active) => (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {active ? (
            <rect
              x="6"
              y="5"
              width="12"
              height="15"
              rx="2"
              fill="currentColor"
              opacity="0.18"
            />
          ) : null}
          <rect x="6" y="5" width="12" height="15" rx="2" />
          <path d="M9 9h6M9 13h6M15 3v4M9 3v4" />
        </svg>
      ),
    },
    {
      key: 'text',
      label: '文本',
      count: counts.text,
      icon: (active) => (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {active ? (
            <path d="M5 6h14v2H13v9h-2V8H5z" fill="currentColor" />
          ) : (
            <path d="M6 7h12M12 7v10M9 17h6" />
          )}
        </svg>
      ),
    },
    {
      key: 'image',
      label: '图像',
      count: counts.image,
      icon: (active) => (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {active ? (
            <rect
              x="4.5"
              y="5.5"
              width="15"
              height="13"
              rx="2"
              fill="currentColor"
              opacity="0.18"
            />
          ) : null}
          <rect x="4.5" y="5.5" width="15" height="13" rx="2" />
          <circle
            cx="10"
            cy="10"
            r="1.25"
            fill={active ? 'currentColor' : 'none'}
          />
          <path d="M7.5 16l3.2-3.3 2.7 2.6 2-2.1 2.3 2.8" />
        </svg>
      ),
    },
    {
      key: 'file',
      label: '文件',
      count: counts.file,
      icon: (active) => (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {active ? (
            <path
              d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z"
              fill="currentColor"
              opacity="0.18"
            />
          ) : null}
          <path d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z" />
          <path d="M14 4.5v4h4" />
        </svg>
      ),
    },
    {
      key: 'favorite',
      label: '收藏',
      count: counts.favorite,
      icon: (active) => (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z"
            fill={active ? 'currentColor' : 'none'}
          />
        </svg>
      ),
    },
  ];
}
