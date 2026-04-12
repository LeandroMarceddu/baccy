import { writable } from "svelte/store";

export interface Preferences {
  trendingInterval: number;
  autoRefresh: boolean;
  showStatusBar: boolean;
  confirmPropertyWrite: boolean;
}

function loadPreferences(): Preferences {
  return {
    trendingInterval: parseInt(localStorage.getItem('trendingInterval') || '5'),
    autoRefresh: localStorage.getItem('autoRefresh') !== 'false',
    showStatusBar: localStorage.getItem('showStatusBar') !== 'false',
    confirmPropertyWrite: localStorage.getItem('confirmPropertyWrite') !== 'false',
  };
}

function createPreferencesStore() {
  const { subscribe, set, update } = writable<Preferences>(loadPreferences());

  return {
    subscribe,
    save: (prefs: Preferences) => {
      localStorage.setItem('trendingInterval', prefs.trendingInterval.toString());
      localStorage.setItem('autoRefresh', prefs.autoRefresh.toString());
      localStorage.setItem('showStatusBar', prefs.showStatusBar.toString());
      localStorage.setItem('confirmPropertyWrite', prefs.confirmPropertyWrite.toString());
      set(prefs);
    },
    reload: () => {
      set(loadPreferences());
    }
  };
}

export const preferences = createPreferencesStore();
