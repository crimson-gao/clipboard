import { useEffect, useState } from 'react';

export function useDebouncedValue<T>(value: T, delay: number, enabled = true) {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    const timer = window.setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      window.clearTimeout(timer);
    };
  }, [delay, enabled, value]);

  useEffect(() => {
    if (enabled) {
      return;
    }

    setDebouncedValue(value);
  }, [enabled, value]);

  return debouncedValue;
}
