// Layout persistence utilities

export interface LayoutState {
  leftPanelSize: number;
  rightPanelSize: number;
  bottomPanelSize: number;
}

const LAYOUT_STORAGE_KEY = 'baccy-layout-state';

const DEFAULT_LAYOUT: LayoutState = {
  leftPanelSize: 20,
  rightPanelSize: 30,
  bottomPanelSize: 30,
};

export function saveLayoutState(state: LayoutState) {
  try {
    localStorage.setItem(LAYOUT_STORAGE_KEY, JSON.stringify(state));
  } catch (error) {
    console.error('Failed to save layout state:', error);
  }
}

export function loadLayoutState(): LayoutState {
  try {
    const stored = localStorage.getItem(LAYOUT_STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      // Validate the loaded state
      return {
        leftPanelSize: validateSize(parsed.leftPanelSize, DEFAULT_LAYOUT.leftPanelSize),
        rightPanelSize: validateSize(parsed.rightPanelSize, DEFAULT_LAYOUT.rightPanelSize),
        bottomPanelSize: validateSize(parsed.bottomPanelSize, DEFAULT_LAYOUT.bottomPanelSize),
      };
    }
  } catch (error) {
    console.error('Failed to load layout state:', error);
  }
  return DEFAULT_LAYOUT;
}

export function resetLayoutState() {
  try {
    localStorage.removeItem(LAYOUT_STORAGE_KEY);
  } catch (error) {
    console.error('Failed to reset layout state:', error);
  }
  return DEFAULT_LAYOUT;
}

function validateSize(value: any, defaultValue: number): number {
  const num = Number(value);
  if (isNaN(num) || num < 10 || num > 90) {
    return defaultValue;
  }
  return num;
}

// Debounce helper for saving layout changes
export function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  
  return function(...args: Parameters<T>) {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => func(...args), wait);
  };
}
