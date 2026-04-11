import Store from 'electron-store';

type Preferences = {
  isPinned: boolean;
  shortcut: string;
};

const defaults: Preferences = {
  isPinned: false,
  shortcut: 'CommandOrControl+Shift+V',
};

export const preferenceStore = new Store<Preferences>({
  defaults,
});

export function getPreferences(): Preferences {
  return {
    isPinned: preferenceStore.get('isPinned'),
    shortcut: preferenceStore.get('shortcut'),
  };
}
