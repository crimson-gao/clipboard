import { useState } from 'react';
import hotkeys from 'hotkeys-js';

const SPECIAL_KEY_LABELS: Record<string, string> = {
  arrowup: 'Up',
  arrowdown: 'Down',
  arrowleft: 'Left',
  arrowright: 'Right',
  escape: 'Escape',
  enter: 'Enter',
  tab: 'Tab',
  space: 'Space',
  ' ': 'Space',
  backspace: 'Backspace',
  delete: 'Delete',
  insert: 'Insert',
  home: 'Home',
  end: 'End',
  pageup: 'PageUp',
  pagedown: 'PageDown',
};

type ShortcutRecorderProps = {
  disabled: boolean;
  onChange: (value: string) => void;
  placeholder: string;
  value: string;
};

function hasModifier(event: React.KeyboardEvent<HTMLInputElement>): boolean {
  return event.metaKey || event.ctrlKey || event.altKey || event.shiftKey;
}

function isModifierOnly(key: string): boolean {
  return ['meta', 'control', 'ctrl', 'alt', 'shift'].includes(key);
}

function normalizePrimaryKey(event: React.KeyboardEvent<HTMLInputElement>): string | null {
  const key = event.key.toLowerCase();

  if (isModifierOnly(key)) {
    return null;
  }

  if (/^f\d{1,2}$/.test(key)) {
    return key.toUpperCase();
  }

  if (SPECIAL_KEY_LABELS[key]) {
    return SPECIAL_KEY_LABELS[key];
  }

  if (key.length === 1 && /[a-z0-9]/.test(key)) {
    return key.toUpperCase();
  }

  if (hotkeys.keyMap[key]) {
    return key.length === 1 ? key : key[0].toUpperCase() + key.slice(1);
  }

  return null;
}

function serializeShortcut(event: React.KeyboardEvent<HTMLInputElement>): string | null {
  const primaryKey = normalizePrimaryKey(event);
  if (!primaryKey) {
    return null;
  }

  const parts: string[] = [];

  if (event.metaKey) {
    parts.push('CommandOrControl');
  }

  if (event.ctrlKey && !event.metaKey) {
    parts.push('CommandOrControl');
  }

  if (event.ctrlKey && event.metaKey) {
    parts.push('Control');
  }

  if (event.altKey) {
    parts.push('Alt');
  }

  if (event.shiftKey) {
    parts.push('Shift');
  }

  parts.push(primaryKey);
  return parts.join('+');
}

export function ShortcutRecorder({
  disabled,
  onChange,
  placeholder,
  value,
}: ShortcutRecorderProps) {
  const [isRecording, setIsRecording] = useState(false);

  const displayValue = isRecording ? '按下新的快捷键组合' : value;

  return (
    <div className="shortcut-recorder">
      <input
        type="text"
        value={displayValue}
        readOnly
        disabled={disabled}
        placeholder={placeholder}
        className={`shortcut-recorder-input${isRecording ? ' is-recording' : ''}`}
        onFocus={() => {
          if (!disabled) {
            setIsRecording(true);
          }
        }}
        onBlur={() => {
          setIsRecording(false);
        }}
        onKeyDown={(event) => {
          if (disabled) {
            return;
          }

          event.preventDefault();
          event.stopPropagation();

          const key = event.key.toLowerCase();

          if (key === 'escape') {
            setIsRecording(false);
            event.currentTarget.blur();
            return;
          }

          if (!hasModifier(event) && ['backspace', 'delete'].includes(key)) {
            onChange('');
            setIsRecording(false);
            return;
          }

          const nextShortcut = serializeShortcut(event);
          if (!nextShortcut) {
            return;
          }

          onChange(nextShortcut);
          setIsRecording(false);
          event.currentTarget.blur();
        }}
      />
      <div className="shortcut-recorder-footer">
        <span className={`shortcut-recorder-badge${isRecording ? ' is-recording' : ''}`}>
          {isRecording ? '录制中' : '点击后录制'}
        </span>
        <span className="shortcut-recorder-hint">
          `Esc` 取消，`Delete` 清空
        </span>
      </div>
    </div>
  );
}
