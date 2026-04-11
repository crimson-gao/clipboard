import { useEffect, useRef, useState } from 'react';

import { clipboardApi } from '../../lib/clipboard-api';
import type { ClipItem } from '../../types';

const imagePreviewUrlCache = new Map<string, string>();

function buildImagePreviewCacheKey(clip: ClipItem): string {
  return `${clip.id}:${clip.updatedAt}`;
}

export function ClipImagePreview({ clip }: { clip: ClipItem }) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [shouldLoad, setShouldLoad] = useState(() =>
    imagePreviewUrlCache.has(buildImagePreviewCacheKey(clip)),
  );
  const [imageUrl, setImageUrl] = useState<string | null>(
    () => imagePreviewUrlCache.get(buildImagePreviewCacheKey(clip)) ?? null,
  );

  useEffect(() => {
    const cacheKey = buildImagePreviewCacheKey(clip);
    if (imagePreviewUrlCache.has(cacheKey)) {
      setShouldLoad(true);
      return;
    }

    const node = containerRef.current;
    if (!node) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries[0]?.isIntersecting) {
          return;
        }

        setShouldLoad(true);
        observer.disconnect();
      },
      {
        root: document.querySelector('.history-scroll'),
        rootMargin: '240px 0px',
        threshold: 0.01,
      },
    );

    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [clip]);

  useEffect(() => {
    if (!shouldLoad) {
      return;
    }

    const cacheKey = buildImagePreviewCacheKey(clip);
    const cached = imagePreviewUrlCache.get(cacheKey);
    if (cached) {
      setImageUrl(cached);
      return;
    }

    let disposed = false;
    void clipboardApi.getClipImagePreview(clip.id).then((payload) => {
      if (!payload || disposed) {
        return;
      }

      const nextUrl = URL.createObjectURL(
        new Blob([new Uint8Array(payload.bytes)], { type: payload.mimeType }),
      );
      imagePreviewUrlCache.set(cacheKey, nextUrl);
      if (!disposed) {
        setImageUrl(nextUrl);
      }
    });

    return () => {
      disposed = true;
    };
  }, [clip, shouldLoad]);

  if (!imageUrl) {
    return (
      <div
        ref={containerRef}
        className="image-preview image-preview-placeholder"
        aria-hidden="true"
      />
    );
  }

  return (
    <div ref={containerRef} className="image-preview">
      <img src={imageUrl} alt="" draggable={false} loading="lazy" />
    </div>
  );
}
