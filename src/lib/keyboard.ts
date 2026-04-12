// Keyboard shortcuts handler

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: () => void;
  description: string;
}

export class KeyboardShortcutManager {
  private shortcuts: Map<string, KeyboardShortcut> = new Map();

  register(shortcut: KeyboardShortcut) {
    const key = this.getShortcutKey(shortcut);
    this.shortcuts.set(key, shortcut);
  }

  unregister(key: string, ctrl?: boolean, shift?: boolean, alt?: boolean) {
    const shortcutKey = this.getShortcutKey({ key, ctrl, shift, alt } as KeyboardShortcut);
    this.shortcuts.delete(shortcutKey);
  }

  handleKeyDown(event: KeyboardEvent): boolean {
    const key = event.key.toLowerCase();
    const shortcutKey = this.getShortcutKey({
      key,
      ctrl: event.ctrlKey || event.metaKey, // Support Cmd on macOS
      shift: event.shiftKey,
      alt: event.altKey,
    } as KeyboardShortcut);

    const shortcut = this.shortcuts.get(shortcutKey);
    if (shortcut) {
      event.preventDefault();
      shortcut.action();
      return true;
    }
    return false;
  }

  getShortcuts(): KeyboardShortcut[] {
    return Array.from(this.shortcuts.values());
  }

  private getShortcutKey(shortcut: { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean }): string {
    const parts: string[] = [];
    if (shortcut.ctrl) parts.push('ctrl');
    if (shortcut.shift) parts.push('shift');
    if (shortcut.alt) parts.push('alt');
    parts.push(shortcut.key.toLowerCase());
    return parts.join('+');
  }
}

// Global keyboard shortcut manager instance
export const keyboardManager = new KeyboardShortcutManager();

// Helper to format shortcut for display
export function formatShortcut(shortcut: KeyboardShortcut): string {
  const parts: string[] = [];
  const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
  
  if (shortcut.ctrl) parts.push(isMac ? '⌘' : 'Ctrl');
  if (shortcut.shift) parts.push(isMac ? '⇧' : 'Shift');
  if (shortcut.alt) parts.push(isMac ? '⌥' : 'Alt');
  parts.push(shortcut.key.toUpperCase());
  
  return parts.join(isMac ? '' : '+');
}
